use anyhow::Context;
use ironposh_client_core::connector::{
    auth_sequence::AuthSequence,
    authenticator::SecContextMaybeInit,
    conntion_pool::{AuthenticatedHttpChannel, PostConAuthSequence, SecContextInited},
    http::{HttpRequestAction, HttpResponse},
};
use tracing::{info, info_span, instrument};

use crate::connection::HttpClient;
use crate::kerberos::send_packet;

/// Handles authentication sequences for both connection establishment and active sessions
pub struct AuthHandler;

impl AuthHandler {
    /// Handles the complete authentication sequence for a connection
    #[instrument(
        name = "auth.sequence",
        level = "info",
        skip(client, auth_sequence),
        fields(sequence_type = "connection"),
        err
    )]
    pub fn handle_auth_sequence(
        client: &dyn HttpClient,
        mut auth_sequence: PostConAuthSequence,
    ) -> Result<(AuthenticatedHttpChannel, HttpRequestAction), anyhow::Error> {
        let _span = info_span!("auth.sequence.handle").entered();
        info!("starting authentication sequence");

        let mut auth_response = None;

        loop {
            let sec_context_init =
                Self::process_security_context(&mut auth_sequence, auth_response.as_ref())?;
            let action = auth_sequence.process_sec_ctx_init(sec_context_init)?;

            match action {
                SecContextInited::Continue { request, sequence } => {
                    info!("continuing authentication sequence");
                    auth_sequence = sequence;

                    auth_response = Some(client.send_request(request)?.response().clone());
                }
                SecContextInited::SendRequest {
                    request,
                    authenticated_http_channel_cert,
                } => {
                    info!("authentication sequence complete, sending final request");

                    info!("authentication sequence successful");
                    return Ok((authenticated_http_channel_cert, request));
                }
            }
        }
    }

    /// Processes the security context initialization, handling both direct initialization
    /// and generator-based flows (for Kerberos KDC communication)
    #[instrument(
        name = "auth.sec_context",
        level = "info",
        skip(auth_sequence, auth_response),
        err
    )]
    fn process_security_context(
        auth_sequence: &mut PostConAuthSequence,
        auth_response: Option<&HttpResponse>,
    ) -> Result<ironposh_client_core::connector::authenticator::SecContextInit, anyhow::Error> {
        let _span = info_span!("auth.sec_context.process").entered();

        let (sequence, mut holder) = auth_sequence.prepare();
        let sec_context_init = sequence.try_init_sec_context(auth_response, &mut holder)?;

        match sec_context_init {
            SecContextMaybeInit::RunGenerator {
                mut packet,
                mut generator_holder,
            } => {
                info!("running generator for KDC communication");

                loop {
                    let kdc_response = send_packet(packet)
                        .context("failed to send packet to KDC during authentication")?;

                    match AuthSequence::resume(generator_holder, kdc_response)? {
                        SecContextMaybeInit::RunGenerator {
                            packet: packet2,
                            generator_holder: generator2,
                        } => {
                            // Continue the generator loop
                            packet = packet2;
                            generator_holder = generator2;
                        }
                        SecContextMaybeInit::Initialized(sec_context_init) => {
                            info!("KDC communication complete, security context initialized");
                            break Ok(sec_context_init);
                        }
                    }
                }
            }
            SecContextMaybeInit::Initialized(sec_context_init) => {
                info!("security context initialized directly");
                Ok(sec_context_init)
            }
        }
    }
}

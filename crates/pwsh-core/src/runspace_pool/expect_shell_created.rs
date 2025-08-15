use protocol_winrm::soap::SoapEnvelope;
use xml::parser::XmlDeserialize;

use super::pool::RunspacePool;

#[derive(Debug)]
pub struct ExpectShellCreated {
    pub(super) runspace_pool: RunspacePool,
}

impl ExpectShellCreated {
    pub fn accept(self, response: String) -> Result<RunspacePool, crate::PwshCoreError> {
        let ExpectShellCreated { mut runspace_pool } = self;

        let parsed = xml::parser::parse(response.as_str())?;

        let soap_response = SoapEnvelope::from_node(parsed.root_element())
            .map_err(crate::PwshCoreError::XmlParsingError)?;

        runspace_pool.shell.accept_create_response(&soap_response)?;

        Ok(runspace_pool)
    }
}
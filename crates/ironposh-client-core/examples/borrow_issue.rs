struct Builder<'a> {
    _data: &'a mut Vec<u8>,
}

// This simulates the Generator that borrows from the Builder
struct Generator<'a> {
    _builder_ref: &'a mut Builder<'a>,
}

// This simulates our AuthContext that owns the data
struct Context {
    data: Vec<u8>,
}

// This simulates the holder for the builder (like our builder_holder)
type BuilderHolder<'a> = Option<Builder<'a>>;

// This function simulates try_init_sec_context
fn create_generator<'ctx, 'holder>(
    context: &'ctx mut Context,
    holder: &'holder mut BuilderHolder<'ctx>,
) -> Generator<'ctx>
where
    'holder: 'ctx, // holder must outlive the borrow from context
{
    // Create a builder that borrows from context
    let builder = Builder {
        _data: &mut context.data,
    };

    // Store builder in holder
    *holder = Some(builder);

    // Create generator that borrows from the builder in holder
    Generator {
        _builder_ref: holder.as_mut().unwrap(),
    }
}

fn main() {
    // This fails - actual loop (uncomment to see the error)
    {
        let mut holder = None;
        // UNCOMMENT TO SEE THE ERROR:
        let mut context = Context {
            data: vec![1, 2, 3],
        };
        let mut iteration = 0;

        loop {
            iteration += 1;
            println!("Iteration {iteration}");

            // This fails on second iteration!
            // The generator from iteration 1 still "holds" the borrows
            {
                let _gen = create_generator(&mut context, &mut holder);
            }

            holder = None; // Clear the holder to simulate breaking the loop
            if iteration >= 10 {
                break;
            }
        }
    }
}

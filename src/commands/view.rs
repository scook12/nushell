use crate::errors::ShellError;
use crate::prelude::*;
use prettyprint::PrettyPrinter;

pub fn view(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.positional.len() == 0 {
        if let Some(span) = args.name_span {
            return Err(ShellError::labeled_error(
                "View requires a filename",
                "needs parameter",
                span,
            ));
        } else {
            return Err(ShellError::string("view requires a filename."));
        }
    }

    let target = match args.positional[0].as_string() {
        Ok(s) => s.clone(),
        Err(e) => {
            if let Some(span) = args.name_span {
                return Err(ShellError::labeled_error(
                    "Expected a string",
                    "not a filename",
                    span,
                ));
            } else {
                return Err(e);
            }
        }
    };

    let cwd = args.env.lock().unwrap().cwd().to_path_buf();

    let printer = PrettyPrinter::default()
        .line_numbers(false)
        .header(false)
        .grid(false)
        .build()
        .map_err(|e| ShellError::string(e))?;

    let file = cwd.join(target);

    let _ = printer.file(file.display().to_string());

    Ok(VecDeque::new().boxed())
}
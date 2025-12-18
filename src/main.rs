mod printer;

use crate::printer::Printer;
use std::io;
use std::io::Write;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let printer = Printer::connect(printer::Models::YHK).await?;
    printer.init().await?;

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        let bytes = io::stdin().read_line(&mut input)?;

        let text = input.trim();

        if bytes == 0 {
            break;
        }
        if text.is_empty() {
            printer.stop_print_sequence().await?;
            continue;
        }

        if text.starts_with(':') {
            match text {
                ":q" => break,
                ":test" => {
                    let img = image::open("test.jpg")?;
                    printer.print_image(img).await?;
                    continue;
                }
                ":feed" => {
                    printer.stop_print_sequence().await?;
                    continue;
                }
                &_ => {
                    println!("Unknown command!!!");
                    printer.print_text("!!!!!!!!!!\nUnknown command").await?;
                    continue;
                }
            }
        }

        printer.print_text(text).await?;
    }

    printer.stop_print_sequence().await?;
    printer.disconnect().await?;
    Ok(())
}

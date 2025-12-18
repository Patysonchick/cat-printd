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
        io::stdin().read_line(&mut input)?;

        let text = input.trim();
        if text.is_empty() {
            continue;
        }

        if text == ":q" {
            break;
        } else if text == ":test" {
            let img = image::open("test.jpg")?;
            printer.print_image(img).await?;
        }

        // println!("Printing...");
        printer.print_text(text).await?;
    }

    printer.disconnect().await?;
    Ok(())
}

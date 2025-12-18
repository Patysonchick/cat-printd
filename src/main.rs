mod printer;

use crate::printer::{Printer, text_to_image};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let printer = Printer::connect(printer::Models::YHK).await?;
    printer.init().await?;

    // let img = image::open("test.jpg")?;
    // printer.print_image(img).await?;

    let text_img = text_to_image("----------");
    printer.print_image(text_img).await?;

    let text_img = text_to_image("123test\nqwe");
    printer.print_image(text_img).await?;

    let text_img = text_to_image("__________");
    printer.print_image(text_img).await?;

    printer.disconnect().await?;
    Ok(())
}

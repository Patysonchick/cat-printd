mod printer;

use crate::printer::Printer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let printer = Printer::connect(printer::Models::YHK).await?;
    printer.init().await?;

    let img = image::open("test.png")?;
    printer.print_image(img).await?;

    printer.disconnect().await?;
    Ok(())
}

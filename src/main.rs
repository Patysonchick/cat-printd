mod printer;

use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::Manager;
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

use crate::printer::Printer;

const YHK_WRITE_CHAR_UUID: &str = "49535343-8841-43f4-a8d4-ecbe34729bb3";
const PRINTER_WIDTH_BYTES: usize = 48; // 384 dots / 8 bits = 48 bytes

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let printer = Printer::connect(printer::Models::YHK).await?;
    printer.init().await?;

    printer.start_print_sequence().await?;

    let total_lines: u8 = 100;
    for i in 0..total_lines {
        // Если печатает негатив, замените 0xFF на 0x00
        let byte_val: u8 = if i % 2 == 0 { 0xFF } else { 0x00 };
        let line_data = [byte_val; PRINTER_WIDTH_BYTES];

        // Формат: [GS v 0] [m] [xL] [xH] [yL] [yH] [DATA]
        let mut packet: [u8; 8 + PRINTER_WIDTH_BYTES] = [0x00; 8 + PRINTER_WIDTH_BYTES];

        packet[0] = 0x1d;
        packet[1] = 0x76;
        packet[2] = 0x30;
        packet[3] = 0x00;
        packet[4] = PRINTER_WIDTH_BYTES as u8; // xL (48 байт)
        packet[5] = 0x00; // xH
        packet[6] = 0x01; // yL (Высота = 1 строка)
        packet[7] = 0x00; // yH
        for i in 0..PRINTER_WIDTH_BYTES {
            packet[i + 8] = line_data[i];
        }

        let chars = printer.peripheral.characteristics();
        let write_char = chars
            .iter()
            .find(|c| c.uuid == Uuid::parse_str(YHK_WRITE_CHAR_UUID).unwrap())
            .expect("Characteristic not found");

        for chunk in packet.chunks(20) {
            printer
                .peripheral
                .write(write_char, chunk, WriteType::WithoutResponse)
                .await?;
        }

        // 20 мс обычно достаточно. Если печатает с пропусками, увеличь до 30.
        time::sleep(Duration::from_millis(30)).await;
    }

    printer.stop_print_sequence().await?;

    println!("Done!");
    printer.disconnect().await?;
    Ok(())
}

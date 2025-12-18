use ab_glyph::{FontRef, PxScale};
use btleplug::{
    api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType},
    platform::{Manager, Peripheral},
};
use image::{DynamicImage, GrayImage, Luma};
use imageproc::drawing::draw_text_mut;
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

#[allow(unused)]
const YHK_SERVICE_UUID: &str = "49535343-fe7d-4ae5-8fa9-9fafd205e455";
const YHK_WRITE_CHAR_UUID: &str = "49535343-8841-43f4-a8d4-ecbe34729bb3";
const YHK_WIDTH: u16 = 384;
const YHK_BYTES: u8 = (384 / 8) as u8; // 48 bytes

pub struct Printer {
    #[allow(unused)]
    model: Models,
    peripheral: Peripheral,
}

impl Printer {
    pub async fn connect(model: Models) -> Result<Self, Box<dyn std::error::Error>> {
        // TODO! make model specific connection, now only YHK-****

        let manager = Manager::new().await.unwrap();
        let adapters = manager.adapters().await?;
        let central = adapters.into_iter().nth(0).expect("No BLE adapter");

        println!("Scanning...");
        central.start_scan(ScanFilter::default()).await?;
        time::sleep(Duration::from_secs(1)).await; // TODO! test diff times

        // TODO! колхоз, переписывать
        let mut peripheral = None;
        for p in central.peripherals().await.unwrap() {
            if p.properties()
                .await
                .unwrap()
                .unwrap()
                .local_name
                .iter()
                .any(|name| name.contains("YHK-"))
            {
                peripheral = Some(p);
                break;
            }
        }
        let peripheral = peripheral.unwrap();

        println!("Connecting to {}...", peripheral.address());
        peripheral.connect().await?;

        let printer = Printer { model, peripheral };
        Ok(printer)
    }

    pub async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Disconnecting...");
        self.peripheral.disconnect().await?;
        Ok(())
    }

    pub async fn init(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.peripheral.discover_services().await?;

        let chars = self.peripheral.characteristics();
        let write_char = chars
            .iter()
            .find(|c| c.uuid == Uuid::parse_str(YHK_WRITE_CHAR_UUID).unwrap())
            .expect("Characteristic not found");

        println!("Init...");
        self.peripheral
            .write(write_char, &[0x1b, 0x40], WriteType::WithoutResponse)
            .await?;
        time::sleep(Duration::from_millis(500)).await;

        Ok(())
    }

    pub async fn start_print_sequence(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Start print sequence...");
        let chars = self.peripheral.characteristics();
        let write_char = chars
            .iter()
            .find(|c| c.uuid == Uuid::parse_str(YHK_WRITE_CHAR_UUID).unwrap())
            .expect("Characteristic not found");

        self.peripheral
            .write(
                write_char,
                &[0x1d, 0x49, 0xf0, 0x19],
                WriteType::WithoutResponse,
            )
            .await?;
        time::sleep(Duration::from_millis(500)).await;

        Ok(())
    }

    pub async fn stop_print_sequence(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Stop print sequence...");
        let chars = self.peripheral.characteristics();
        let write_char = chars
            .iter()
            .find(|c| c.uuid == Uuid::parse_str(YHK_WRITE_CHAR_UUID).unwrap())
            .expect("Characteristic not found");

        self.peripheral
            .write(
                write_char,
                &[0x0a, 0x0a, 0x0a, 0x0a],
                WriteType::WithoutResponse,
            )
            .await?;
        time::sleep(Duration::from_secs(1)).await;

        Ok(())
    }

    pub async fn print_line(&self, line_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        // Если печатает негатив, замените 0xFF на 0x00
        // let byte_val: u8 = if i % 2 == 0 { 0xFF } else { 0x00 };
        // let line_data = [byte_val; YHK_WIDTH_BYTES];

        // Формат: [GS v 0] [m] [xL] [xH] [yL] [yH] [DATA]
        let mut packet: [u8; 8 + YHK_BYTES as usize] = [0; 8 + YHK_BYTES as usize];

        packet[0] = 0x1d;
        packet[1] = 0x76;
        packet[2] = 0x30;
        packet[3] = 0x00;
        packet[4] = YHK_BYTES; // xL (48 байт)
        packet[5] = 0x00; // xH
        packet[6] = 0x01; // yL (Высота = 1 строка)
        packet[7] = 0x00; // yH
        for i in 0..YHK_BYTES {
            packet[(i + 8) as usize] = line_data[i as usize];
        }

        let chars = self.peripheral.characteristics();
        let write_char = chars
            .iter()
            .find(|c| c.uuid == Uuid::parse_str(YHK_WRITE_CHAR_UUID).unwrap())
            .expect("Characteristic not found");

        // for chunk in packet.chunks(20) {
        //     self.peripheral
        //         .write(write_char, chunk, WriteType::WithoutResponse)
        //         .await?;
        // }
        self.peripheral
            .write(write_char, &packet, WriteType::WithoutResponse)
            .await?;

        time::sleep(Duration::from_millis(20)).await;

        Ok(())
    }

    pub async fn print_image(&self, img: DynamicImage) -> Result<(), Box<dyn std::error::Error>> {
        let img = img.resize(
            YHK_WIDTH as u32,
            u32::MAX,
            image::imageops::FilterType::Nearest,
        );
        let img_gray = img.to_luma8();

        println!("Printing...");
        self.start_print_sequence().await?;

        for y in 0..img_gray.height() {
            let mut line_data = [0u8; YHK_BYTES as usize];

            for x in 0..YHK_WIDTH {
                if x >= img_gray.width() as u16 {
                    continue;
                }

                let pixel = img_gray.get_pixel(x as u32, y);
                let brightness = pixel[0]; // 0..255

                // Если картинка инвертирована, поменяй знак (< на >).
                if brightness < 128 {
                    let byte_idx = (x / 8) as usize;
                    let bit_idx = 7 - (x % 8); // Старший бит слева
                    line_data[byte_idx] |= 1 << bit_idx;
                }
            }

            self.print_line(&line_data).await?;
        }

        // self.stop_print_sequence().await?;
        println!("Done!");
        Ok(())
    }

    pub async fn print_text(&self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        let img = text_to_image(text);

        self.print_image(img).await?;
        Ok(())
    }
}

#[allow(clippy::upper_case_acronyms)]
pub enum Models {
    YHK,
}

const MAX_TEXT_WIDTH: u32 = 384;
const FONT_SIZE: f32 = 24.0;
// const PADDING: u32 = 10;
// const LINE_SPACING: u32 = 4;

fn text_to_image(text: &str) -> DynamicImage {
    let font_data =
        include_bytes!("../font/JetBrainsMonoNerdFont/JetBrainsMonoNerdFont-Regular.ttf");
    let font = FontRef::try_from_slice(font_data).expect("Error constructing Font");
    let scale = PxScale::from(FONT_SIZE);

    let text_lines = text.lines();
    let mut image = GrayImage::from_pixel(
        MAX_TEXT_WIDTH,
        (text_lines.clone().count() as f32 * FONT_SIZE) as u32,
        Luma([255u8]),
    );

    for (i, line) in text_lines.enumerate() {
        draw_text_mut(
            &mut image,
            Luma([0u8]),
            0,
            (FONT_SIZE * i as f32) as i32,
            scale,
            &font,
            line,
        );
    }

    DynamicImage::ImageLuma8(image)
}

use btleplug::{
    api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType},
    platform::{Manager, Peripheral},
};
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

const YHK_SERVICE_UUID: &str = "49535343-fe7d-4ae5-8fa9-9fafd205e455";
const YHK_WRITE_CHAR_UUID: &str = "49535343-8841-43f4-a8d4-ecbe34729bb3";

pub struct Printer {
    model: Models,
    pub peripheral: Peripheral, // TODO! вернуть в приватный
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
        let cmd_init = [0x1b, 0x40];
        self.peripheral
            .write(write_char, &cmd_init, WriteType::WithoutResponse)
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

        let cmd_start = [0x1d, 0x49, 0xf0, 0x19];
        self.peripheral
            .write(write_char, &cmd_start, WriteType::WithoutResponse)
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

        let cmd_end = vec![0x0a, 0x0a, 0x0a, 0x0a];
        self.peripheral
            .write(write_char, &cmd_end, WriteType::WithoutResponse)
            .await?;
        time::sleep(Duration::from_secs(1)).await;

        Ok(())
    }
}

pub enum Models {
    YHK,
}

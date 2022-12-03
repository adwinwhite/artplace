use std::io::Result;
use prost_build::Config;
fn main() -> Result<()> {
    Config::new()
        // .type_attribute("artplace.wsmsg.SetBrush",
                        // "#[derive(actix::Message)] #[rtype(result=\"()\")]")
        // .type_attribute("artplace.wsmsg.Movement",
                        // "#[derive(actix::Message)] #[rtype(result=\"()\")]")
        // .type_attribute("artplace.wsmsg.JoinRoom",
                        // "#[derive(actix::Message)] #[rtype(artplace::wsmsg::RoomInit)]")
        // .type_attribute("artplace.wsmsg.Snapshot",
                        // "#[derive(actix::Message)] #[rtype(result=\"()\")]")
        // .type_attribute("artplace.wsmsg.SnapperRequest",
                        // "#[derive(actix::Message)] #[rtype(result=\"()\")]")
        .type_attribute("artplace.wsmsg.ServerMessage",
                        "#[derive(actix::Message)] #[rtype(result=\"()\")]")
        .type_attribute("artplace.wsmsg.RoomInit",
                        "#[derive(actix::MessageResponse)]")
        .compile_protos(&["./src/messages.proto"], &["src/"])?;
    Ok(())
}

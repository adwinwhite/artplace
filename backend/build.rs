use std::io::Result;
use prost_build::Config;
fn main() -> Result<()> {
    Config::new()
        // .type_attribute("artplace.messages.Brush",
                        // "#[derive(actix::Message)] #[rtype")
        // .type_attribute("artplace.messages.Brush.PencilBrush",
                        // "#[derive(actix::Message)]")
        // .type_attribute("artplace.messages.Brush.oneofbrushKinds",
                        // "#[derive(actix::Message)]")
        // .type_attribute("artplace.messages.Uid",
                        // "#[derive(actix::MessageResponse, Copy, Eq, Hash)]")
        .type_attribute("artplace.messages.InitClient",
                        "#[derive(actix::MessageResponse)]")
        // .type_attribute("artplace.messages.SetBrush",
                        // "#[derive(actix::Message)]")
        // .type_attribute("artplace.messages.Movement",
                        // "#[derive(actix::Message)]")
        // .type_attribute("artplace.messages.Movement.Pos",
                        // "#[derive(actix::Message)]")
        .type_attribute("artplace.messages.ClientMessage",
                        "#[derive(actix::Message)] #[rtype(result=\"()\")]")
        // .type_attribute("artplace.messages.ClientMessage.oneofmessageKinds",
                        // "#[derive(actix::Message)]")
        .compile_protos(&["./src/messages.proto"], &["src/"])?;
    Ok(())
}

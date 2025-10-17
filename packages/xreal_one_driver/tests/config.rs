use xreal_one_driver::config::Config;

#[test]
fn without_camera() -> Result<(), anyhow::Error> {
    Config::parse(include_bytes!("./data/without_camera.json"))?;
    Ok(())
}

#[test]
fn with_camera() -> Result<(), anyhow::Error> {
    Config::parse(include_bytes!("./data/with_camera.json"))?;
    Ok(())
}

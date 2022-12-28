use raqote::IntPoint;

#[derive(Debug, Clone)]
pub struct Config {
    pub displays: Vec<(String, IntPoint)>,
    pub plugins: Vec<String>
}

pub(crate) fn load() -> Result<Config, String> {
    Ok(Config {
        displays: vec![("display:3/activate".to_owned(), IntPoint::new(0, 0))],
        plugins: vec!["/home/user/plugins/example2.lua".to_owned()]
    })
}

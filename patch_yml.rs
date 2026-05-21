<<<<<<< SEARCH
pub struct ProjConfig {
    #[serde(default)]
    pub directories: HashMap<String, DirConfig>,
}
=======
pub struct ProjConfig {
    #[serde(default)]
    pub directories: HashMap<String, DirConfig>,
    pub build: Option<BuildConfig>,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct BuildConfig {
    pub scripts: Option<Vec<String>>,
}
>>>>>>> REPLACE

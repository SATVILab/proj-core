pub fn yml_get() -> String {
    "projr yml content".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yml_get() {
        assert_eq!(yml_get(), "projr yml content");
    }
}

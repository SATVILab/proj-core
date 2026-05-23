import re

with open('src/ignore.rs', 'r') as f:
    content = f.read()

content = content.replace("update_ignores_for(&root, &validated).unwrap();", "update_ignores_for(camino::Utf8Path::from_path(&root).unwrap(), &validated).unwrap();")
content = content.replace("let result = root_from(temp.path()).unwrap();", "let result = root_from(camino::Utf8Path::from_path(temp.path()).unwrap()).unwrap();")
content = content.replace("add_manual_ignores(&root, &[\"temp.log\".to_string()], true, IgnoreType::Git).unwrap();", "add_manual_ignores(camino::Utf8Path::from_path(&root).unwrap(), &[\"temp.log\".to_string()], true, IgnoreType::Git).unwrap();")
content = content.replace("remove_manual_ignores(&root, &[\"!!temp.log\".to_string()], IgnoreType::Git).unwrap();", "remove_manual_ignores(camino::Utf8Path::from_path(&root).unwrap(), &[\"!!temp.log\".to_string()], IgnoreType::Git).unwrap();")

with open('src/ignore.rs', 'w') as f:
    f.write(content)

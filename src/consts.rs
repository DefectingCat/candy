pub static NOT_FOUND: &'static str = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" /> <meta
      name="viewport"
      content="width=device-width, user-scalable=no, initial-scale=1.0, maximum-scale=1.0, minimum-scale=1.0"
    />
    <meta http-equiv="X-UA-Compatible" content="ie=edge" />
    <title>Not found</title>
  </head>
  <body>
    <h1>404</h1>
    <p>Content not found.</p>
  </body>
</html>
"#;

pub static STATIC_FILE_TYPE: [&str; 7] = [".html", ".css", ".js", ".json", ".ico", ".wasm", ".svg"];
pub static IMAGE_FILE: [&str; 3] = [".png", ".jpg", ".ico"];

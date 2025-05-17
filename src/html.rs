pub fn template_html_internal_error(
    error_code: String,
    p1: String,
    p2: String,
    redirect: String,
) -> String {
    static HTML_TEMPLATE: &str = r#"
    <!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Service Unavailable</title>
    <style>
        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background: #f8f9fa;
            color: #343a40;
            margin: 0;
            padding: 0;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            text-align: center;
        }

        .error-container {
            max-width: 500px;
            padding: 2rem;
            background: white;
            border-radius: 10px;
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
        }

        h1 {
            font-size: 3rem;
            margin: 0;
            color: #dc3545;
        }

        .error-code {
            font-size: 1.2rem;
            font-weight: bold;
            margin: 0.5rem 0;
        }

        p {
            margin: 1rem 0;
            line-height: 1.5;
        }

        a {
            color: #007bff;
            text-decoration: none;
        }

        a:hover {
            text-decoration: underline;
        }

        .btn {
            display: inline-block;
            margin-top: 1rem;
            padding: 0.5rem 1rem;
            background: #007bff;
            color: white;
            border-radius: 5px;
            transition: background 0.3s;
        }

        .btn:hover {
            background: #0056b3;
            text-decoration: none;
        }
    </style>
</head>
<body>
    <div class="error-container">
        <h1>Oops!</h1>
        <div class="error-code">ERRORCODE</div>
        <p>P1</p>
        <p>P2</p>
        <a href="url_redirect" class="btn">Refresh Page</a>
    </div>
</body>
</html>
"#;
    let html = String::from(HTML_TEMPLATE).clone();
    html.replace("ERRORCODE", &error_code.as_str())
        .replace("P1", &p1.as_str())
        .replace("P2", &p2.as_str())
        .replace("url_redirect", &redirect.as_str())
}

pub fn template_html_antibot(redirect: String) -> String {
    static HTML_TEMPLATE: &str = r#"
    <!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Service Unavailable</title>
    <style>
        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background: #f8f9fa;
            color: #343a40;
            margin: 0;
            padding: 0;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            text-align: center;
        }

        .error-container {
            max-width: 500px;
            padding: 2rem;
            background: white;
            border-radius: 10px;
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
        }

        h1 {
            font-size: 3rem;
            margin: 0;
            color: #dc3545;
        }

        .error-code {
            font-size: 1.2rem;
            font-weight: bold;
            margin: 0.5rem 0;
        }

        p {
            margin: 1rem 0;
            line-height: 1.5;
        }

        a {
            color: #007bff;
            text-decoration: none;
        }

        a:hover {
            text-decoration: underline;
        }

        .btn {
            display: inline-block;
            margin-top: 1rem;
            padding: 0.5rem 1rem;
            background: #007bff;
            color: white;
            border-radius: 5px;
            transition: background 0.3s;
        }

        .btn:hover {
            background: #0056b3;
            text-decoration: none;
        }
    </style>
</head>
<body>
    <div class="error-container">
        <h1>ANTIBOT process</h1>
        <p></p>
        <p>Clic on refresh</p>
        <a href="PATHREFRESH" class="btn">Refresh Page</a>
    </div>
</body>
</html>
"#;
    let html = String::from(HTML_TEMPLATE).clone();
    html.replace("PATHREFRESH", &redirect.as_str())
}

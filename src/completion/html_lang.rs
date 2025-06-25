// HTML language completion provider

use super::LanguageProvider;

pub struct HtmlProvider;

impl LanguageProvider for HtmlProvider {
    fn keywords(&self) -> &[&'static str] {
        &[
            "html", "head", "title", "body", "div", "span", "p", "a", "img", "ul", "ol", "li",
            "h1", "h2", "h3", "h4", "h5", "h6", "table", "tr", "td", "th", "thead", "tbody",
            "form", "input", "button", "select", "option", "textarea", "label", "fieldset",
            "legend", "nav", "header", "footer", "section", "article", "aside", "main", "figure",
            "figcaption", "audio", "video", "source", "canvas", "svg", "script", "style", "link",
            "meta", "br", "hr", "strong", "em", "b", "i", "u", "small", "mark", "del", "ins",
            "sub", "sup", "blockquote", "cite", "q", "abbr", "address", "time", "code", "pre",
            "kbd", "samp", "var", "dfn", "data", "output", "progress", "meter", "details", "summary",
            "iframe", "embed", "object", "param", "area", "map", "track", "wbr", "ruby", "rt", "rp"
        ]
    }

    fn snippets(&self) -> &[(&'static str, &'static str)] {
        &[
            ("html5", "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n    <meta charset=\"UTF-8\">\n    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n    <title>${1:Document}</title>\n</head>\n<body>\n    ${2:<!-- content -->}\n</body>\n</html>"),
            ("div", "<div${1: class=\"${2:class-name}\"}>\n    ${3:content}\n</div>"),
            ("span", "<span${1: class=\"${2:class-name}\"}>${3:content}</span>"),
            ("p", "<p${1: class=\"${2:class-name}\"}>${3:content}</p>"),
            ("a", "<a href=\"${1:#}\"${2: target=\"${3:_blank}\"}>${4:link text}</a>"),
            ("img", "<img src=\"${1:image.jpg}\" alt=\"${2:description}\"${3: width=\"${4:width}\" height=\"${5:height}\"}>"),
            ("ul", "<ul${1: class=\"${2:class-name}\"}>\n    <li>${3:item 1}</li>\n    <li>${4:item 2}</li>\n</ul>"),
            ("ol", "<ol${1: class=\"${2:class-name}\"}>\n    <li>${3:item 1}</li>\n    <li>${4:item 2}</li>\n</ol>"),
            ("li", "<li${1: class=\"${2:class-name}\"}>${3:content}</li>"),
            ("table", "<table${1: class=\"${2:class-name}\"}>\n    <thead>\n        <tr>\n            <th>${3:Header 1}</th>\n            <th>${4:Header 2}</th>\n        </tr>\n    </thead>\n    <tbody>\n        <tr>\n            <td>${5:Data 1}</td>\n            <td>${6:Data 2}</td>\n        </tr>\n    </tbody>\n</table>"),
            ("form", "<form${1: action=\"${2:#}\" method=\"${3:post}\"}>\n    ${4:<!-- form elements -->}\n    <button type=\"submit\">${5:Submit}</button>\n</form>"),
            ("input", "<input type=\"${1:text}\" name=\"${2:name}\" id=\"${3:id}\"${4: placeholder=\"${5:placeholder}\"}>"),
            ("button", "<button type=\"${1:button}\"${2: class=\"${3:class-name}\"}>${4:Button Text}</button>"),
            ("select", "<select name=\"${1:name}\" id=\"${2:id}\">\n    <option value=\"${3:value1}\">${4:Option 1}</option>\n    <option value=\"${5:value2}\">${6:Option 2}</option>\n</select>"),
            ("textarea", "<textarea name=\"${1:name}\" id=\"${2:id}\" rows=\"${3:4}\" cols=\"${4:50}\"${5: placeholder=\"${6:placeholder}\"}></textarea>"),
            ("label", "<label for=\"${1:input-id}\">${2:Label Text}</label>"),
            ("h1", "<h1${1: class=\"${2:class-name}\"}>${3:Heading 1}</h1>"),
            ("h2", "<h2${1: class=\"${2:class-name}\"}>${3:Heading 2}</h2>"),
            ("h3", "<h3${1: class=\"${2:class-name}\"}>${3:Heading 3}</h3>"),
            ("nav", "<nav${1: class=\"${2:class-name}\"}>\n    ${3:<!-- navigation links -->}\n</nav>"),
            ("header", "<header${1: class=\"${2:class-name}\"}>\n    ${3:<!-- header content -->}\n</header>"),
            ("footer", "<footer${1: class=\"${2:class-name}\"}>\n    ${3:<!-- footer content -->}\n</footer>"),
            ("section", "<section${1: class=\"${2:class-name}\"}>\n    ${3:<!-- section content -->}\n</section>"),
            ("article", "<article${1: class=\"${2:class-name}\"}>\n    ${3:<!-- article content -->}\n</article>"),
            ("aside", "<aside${1: class=\"${2:class-name}\"}>\n    ${3:<!-- sidebar content -->}\n</aside>"),
            ("main", "<main${1: class=\"${2:class-name}\"}>\n    ${3:<!-- main content -->}\n</main>"),
        ]
    }

    fn get_documentation(&self, keyword: &str) -> String {
        match keyword {
            "html" => "<html> element - Root element of HTML document\n\nUsage: <html lang=\"en\">content</html>".to_string(),
            "head" => "<head> element - Document metadata container\n\nUsage: <head>metadata</head>".to_string(),
            "title" => "<title> element - Document title\n\nUsage: <title>Page Title</title>".to_string(),
            "body" => "<body> element - Document body content\n\nUsage: <body>visible content</body>".to_string(),
            "div" => "<div> element - Generic container\n\nUsage: <div>content</div>".to_string(),
            "span" => "<span> element - Inline container\n\nUsage: <span>text</span>".to_string(),
            "p" => "<p> element - Paragraph\n\nUsage: <p>paragraph text</p>".to_string(),
            "a" => "<a> element - Anchor/link\n\nUsage: <a href=\"url\">link text</a>".to_string(),
            "img" => "<img> element - Image\n\nUsage: <img src=\"image.jpg\" alt=\"description\">".to_string(),
            "ul" => "<ul> element - Unordered list\n\nUsage: <ul><li>item</li></ul>".to_string(),
            "ol" => "<ol> element - Ordered list\n\nUsage: <ol><li>item</li></ol>".to_string(),
            "li" => "<li> element - List item\n\nUsage: <li>list item content</li>".to_string(),
            "table" => "<table> element - Table\n\nUsage: <table><tr><td>cell</td></tr></table>".to_string(),
            "tr" => "<tr> element - Table row\n\nUsage: <tr><td>cell content</td></tr>".to_string(),
            "td" => "<td> element - Table data cell\n\nUsage: <td>cell content</td>".to_string(),
            "th" => "<th> element - Table header cell\n\nUsage: <th>header content</th>".to_string(),
            "thead" => "<thead> element - Table header group\n\nUsage: <thead><tr><th>header</th></tr></thead>".to_string(),
            "tbody" => "<tbody> element - Table body group\n\nUsage: <tbody><tr><td>data</td></tr></tbody>".to_string(),
            "form" => "<form> element - User input form\n\nAttributes: action, method\n\nUsage: <form action=\"#\" method=\"post\">inputs</form>".to_string(),
            "input" => "<input> element - Form input field\n\nAttributes: type, name, value\n\nUsage: <input type=\"text\" name=\"field\">".to_string(),
            "button" => "<button> element - Clickable button\n\nAttributes: type, onclick\n\nUsage: <button type=\"submit\">Submit</button>".to_string(),
            "select" => "<select> element - Dropdown selection\n\nUsage: <select><option>choice</option></select>".to_string(),
            "option" => "<option> element - Selection option\n\nUsage: <option value=\"val\">text</option>".to_string(),
            "textarea" => "<textarea> element - Multi-line text input\n\nAttributes: rows, cols\n\nUsage: <textarea rows=\"4\" cols=\"50\"></textarea>".to_string(),
            "label" => "<label> element - Form field label\n\nUsage: <label for=\"input-id\">Label</label>".to_string(),
            "h1" => "<h1> element - Main heading\n\nUsage: <h1>Main Title</h1>".to_string(),
            "h2" => "<h2> element - Section heading\n\nUsage: <h2>Section Title</h2>".to_string(),
            "h3" => "<h3> element - Subsection heading\n\nUsage: <h3>Subsection Title</h3>".to_string(),
            "h4" => "<h4> element - Sub-subsection heading\n\nUsage: <h4>Sub-subsection Title</h4>".to_string(),
            "h5" => "<h5> element - Minor heading\n\nUsage: <h5>Minor Title</h5>".to_string(),
            "h6" => "<h6> element - Smallest heading\n\nUsage: <h6>Smallest Title</h6>".to_string(),
            "nav" => "<nav> element - Navigation links\n\nUsage: <nav><a href=\"#\">link</a></nav>".to_string(),
            "header" => "<header> element - Page/section header\n\nUsage: <header>header content</header>".to_string(),
            "footer" => "<footer> element - Page/section footer\n\nUsage: <footer>footer content</footer>".to_string(),
            "section" => "<section> element - Document section\n\nUsage: <section>section content</section>".to_string(),
            "article" => "<article> element - Independent content\n\nUsage: <article>article content</article>".to_string(),
            "aside" => "<aside> element - Sidebar content\n\nUsage: <aside>sidebar content</aside>".to_string(),
            "main" => "<main> element - Main content area\n\nUsage: <main>main content</main>".to_string(),
            "figure" => "<figure> element - Self-contained content\n\nUsage: <figure><img><figcaption>caption</figcaption></figure>".to_string(),
            "figcaption" => "<figcaption> element - Figure caption\n\nUsage: <figcaption>Image description</figcaption>".to_string(),
            "video" => "<video> element - Video content\n\nAttributes: controls, src\n\nUsage: <video controls><source src=\"video.mp4\"></video>".to_string(),
            "audio" => "<audio> element - Audio content\n\nAttributes: controls, src\n\nUsage: <audio controls><source src=\"audio.mp3\"></audio>".to_string(),
            "canvas" => "<canvas> element - Drawing surface\n\nAttributes: width, height\n\nUsage: <canvas width=\"300\" height=\"200\"></canvas>".to_string(),
            "script" => "<script> element - JavaScript code\n\nAttributes: src, type\n\nUsage: <script src=\"script.js\"></script>".to_string(),
            "style" => "<style> element - CSS styles\n\nUsage: <style>CSS rules</style>".to_string(),
            "link" => "<link> element - External resource link\n\nAttributes: rel, href\n\nUsage: <link rel=\"stylesheet\" href=\"style.css\">".to_string(),
            "meta" => "<meta> element - Document metadata\n\nAttributes: name, content\n\nUsage: <meta name=\"viewport\" content=\"width=device-width\">".to_string(),
            "br" => "<br> element - Line break\n\nUsage: <br> (self-closing)".to_string(),
            "hr" => "<hr> element - Horizontal rule\n\nUsage: <hr> (self-closing)".to_string(),
            "strong" => "<strong> element - Strong importance\n\nUsage: <strong>important text</strong>".to_string(),
            "em" => "<em> element - Emphasized text\n\nUsage: <em>emphasized text</em>".to_string(),
            "b" => "<b> element - Bold text\n\nUsage: <b>bold text</b>".to_string(),
            "i" => "<i> element - Italic text\n\nUsage: <i>italic text</i>".to_string(),
            "u" => "<u> element - Underlined text\n\nUsage: <u>underlined text</u>".to_string(),
            "code" => "<code> element - Inline code\n\nUsage: <code>code snippet</code>".to_string(),
            "pre" => "<pre> element - Preformatted text\n\nUsage: <pre>preformatted text</pre>".to_string(),
            "blockquote" => "<blockquote> element - Block quotation\n\nUsage: <blockquote>quoted text</blockquote>".to_string(),
            "iframe" => "<iframe> element - Inline frame\n\nAttributes: src, width, height\n\nUsage: <iframe src=\"page.html\"></iframe>".to_string(),
            _ => format!("<{}> - HTML element", keyword),
        }
    }
}

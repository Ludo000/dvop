// JavaScript/TypeScript language completion provider

use super::LanguageProvider;

pub struct JavaScriptProvider;

impl LanguageProvider for JavaScriptProvider {
    fn keywords(&self) -> &[&'static str] {
        &[
            "abstract", "arguments", "boolean", "break", "byte", "case", "catch", "char", "class",
            "const", "continue", "debugger", "default", "delete", "do", "double", "else", "enum",
            "eval", "export", "extends", "false", "final", "finally", "float", "for", "function",
            "goto", "if", "implements", "import", "in", "instanceof", "int", "interface", "let",
            "long", "native", "new", "null", "package", "private", "protected", "public", "return",
            "short", "static", "super", "switch", "synchronized", "this", "throw", "throws",
            "transient", "true", "try", "typeof", "var", "void", "volatile", "while", "with", "yield",
            "console", "document", "window", "Array", "Object", "Promise", "async", "await",
            "setTimeout", "setInterval", "clearTimeout", "clearInterval", "fetch", "JSON",
            "localStorage", "sessionStorage", "getElementById", "querySelector", "addEventListener"
        ]
    }

    fn snippets(&self) -> &[(&'static str, &'static str)] {
        &[
            ("function", "function ${1:name}(${2:parameters}) {\n    ${3:// body}\n}"),
            ("arrow", "(${1:parameters}) => {\n    ${2:// body}\n}"),
            ("class", "class ${1:Name} {\n    constructor(${2:parameters}) {\n        ${3:// constructor}\n    }\n}"),
            ("if", "if (${1:condition}) {\n    ${2:// body}\n}"),
            ("for", "for (${1:let i = 0}; ${2:i < length}; ${3:i++}) {\n    ${4:// body}\n}"),
            ("foreach", "${1:array}.forEach((${2:item}) => {\n    ${3:// body}\n});"),
            ("promise", "new Promise((resolve, reject) => {\n    ${1:// async code}\n});"),
            ("async", "async function ${1:name}(${2:parameters}) {\n    ${3:// async body}\n}"),
            ("try", "try {\n    ${1:// code}\n} catch (${2:error}) {\n    ${3:// error handling}\n}"),
            ("switch", "switch (${1:expression}) {\n    case ${2:value}:\n        ${3:// code}\n        break;\n    default:\n        ${4:// default}\n}"),
            ("while", "while (${1:condition}) {\n    ${2:// body}\n}"),
            ("import", "import { ${1:item} } from '${2:module}';"),
            ("export", "export { ${1:item} } from '${2:module}';"),
            ("const", "const ${1:name} = ${2:value};"),
            ("let", "let ${1:name} = ${2:value};"),
        ]
    }

    fn get_documentation(&self, keyword: &str) -> String {
        match keyword {
            "function" => "function keyword - Define a function\n\nSyntax: function name(params) { body }".to_string(),
            "class" => "class keyword - Define a class\n\nSyntax: class Name { constructor() {} }".to_string(),
            "async" => "async keyword - Define an asynchronous function\n\nSyntax: async function name() { await expr; }".to_string(),
            "await" => "await keyword - Wait for Promise resolution\n\nSyntax: const result = await promise;".to_string(),
            "Promise" => "Promise - Represents asynchronous operations\n\nMethods: then(), catch(), finally()".to_string(),
            "console" => "console object - Provides debugging methods\n\nMethods: log(), error(), warn(), info()".to_string(),
            "const" => "const keyword - Declare constant variable\n\nSyntax: const name = value;".to_string(),
            "let" => "let keyword - Declare block-scoped variable\n\nSyntax: let name = value;".to_string(),
            "var" => "var keyword - Declare function-scoped variable\n\nSyntax: var name = value;".to_string(),
            "if" => "if statement - Conditional execution\n\nSyntax: if (condition) { body }".to_string(),
            "for" => "for loop - Iterate with counter\n\nSyntax: for (init; condition; increment) { body }".to_string(),
            "while" => "while loop - Iterate while condition is true\n\nSyntax: while (condition) { body }".to_string(),
            "try" => "try statement - Error handling\n\nSyntax: try { code } catch (error) { handler }".to_string(),
            "catch" => "catch clause - Handle exceptions\n\nSyntax: catch (error) { handler }".to_string(),
            "throw" => "throw statement - Throw an exception\n\nSyntax: throw new Error('message');".to_string(),
            "switch" => "switch statement - Multi-way branching\n\nSyntax: switch (expr) { case value: break; }".to_string(),
            "import" => "import statement - Import modules\n\nSyntax: import { item } from 'module';".to_string(),
            "export" => "export statement - Export modules\n\nSyntax: export { item } from 'module';".to_string(),
            "Array" => "Array object - Ordered list of values\n\nMethods: push(), pop(), map(), filter(), reduce()".to_string(),
            "Object" => "Object - Base type for all objects\n\nMethods: keys(), values(), entries(), assign()".to_string(),
            "JSON" => "JSON object - Parse and stringify JSON\n\nMethods: parse(), stringify()".to_string(),
            "fetch" => "fetch() - Make HTTP requests\n\nSyntax: fetch(url).then(response => response.json())".to_string(),
            "setTimeout" => "setTimeout() - Execute after delay\n\nSyntax: setTimeout(callback, milliseconds)".to_string(),
            "setInterval" => "setInterval() - Execute repeatedly\n\nSyntax: setInterval(callback, milliseconds)".to_string(),
            "document" => "document object - DOM interface\n\nMethods: getElementById(), querySelector(), createElement()".to_string(),
            "window" => "window object - Global browser object\n\nProperties: location, history, localStorage".to_string(),
            _ => format!("{} - JavaScript language element for building dynamic web applications", keyword),
        }
    }
}

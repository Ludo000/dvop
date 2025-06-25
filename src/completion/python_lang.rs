// Python language completion provider

use super::LanguageProvider;

pub struct PythonProvider;

impl LanguageProvider for PythonProvider {
    fn keywords(&self) -> &[&'static str] {
        &[
            "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class",
            "continue", "def", "del", "elif", "else", "except", "finally", "for", "from", "global",
            "if", "import", "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise",
            "return", "try", "while", "with", "yield", "print", "len", "range", "enumerate",
            "zip", "map", "filter", "sorted", "reversed", "sum", "min", "max", "abs", "round",
            "int", "float", "str", "list", "tuple", "dict", "set", "bool", "type", "isinstance",
            "hasattr", "getattr", "setattr", "delattr", "open", "file", "input", "raw_input",
            "__init__", "__str__", "__repr__", "__len__", "__getitem__", "__setitem__", "__iter__"
        ]
    }

    fn snippets(&self) -> &[(&'static str, &'static str)] {
        &[
            ("def", "def ${1:function_name}(${2:parameters}):\n    ${3:pass}"),
            ("class", "class ${1:ClassName}:\n    def __init__(self${2:, parameters}):\n        ${3:pass}"),
            ("if", "if ${1:condition}:\n    ${2:pass}"),
            ("for", "for ${1:item} in ${2:iterable}:\n    ${3:pass}"),
            ("while", "while ${1:condition}:\n    ${2:pass}"),
            ("try", "try:\n    ${1:pass}\nexcept ${2:Exception} as e:\n    ${3:pass}"),
            ("with", "with ${1:expression} as ${2:variable}:\n    ${3:pass}"),
            ("lambda", "lambda ${1:args}: ${2:expression}"),
            ("list_comp", "[${1:expression} for ${2:item} in ${3:iterable}]"),
            ("dict_comp", "{${1:key}: ${2:value} for ${3:item} in ${4:iterable}}"),
            ("import", "import ${1:module}"),
            ("from_import", "from ${1:module} import ${2:item}"),
            ("main", "if __name__ == '__main__':\n    ${1:pass}"),
            ("docstring", "\"\"\"${1:Description}\n\n    Args:\n        ${2:param}: ${3:description}\n\n    Returns:\n        ${4:description}\n    \"\"\""),
            ("init", "def __init__(self${1:, parameters}):\n    ${2:pass}"),
        ]
    }

    fn get_documentation(&self, keyword: &str) -> String {
        match keyword {
            "def" => "def keyword - Define a function\n\nSyntax: def name(params): body".to_string(),
            "class" => "class keyword - Define a class\n\nSyntax: class Name: def __init__(self): pass".to_string(),
            "import" => "import keyword - Import modules\n\nSyntax: import module or from module import name".to_string(),
            "if" => "if statement - Conditional execution\n\nSyntax: if condition: body".to_string(),
            "for" => "for loop - Iterate over sequence\n\nSyntax: for item in iterable: body".to_string(),
            "while" => "while loop - Execute while condition is true\n\nSyntax: while condition: body".to_string(),
            "try" => "try statement - Exception handling\n\nSyntax: try: code except Exception: handler".to_string(),
            "with" => "with statement - Context manager\n\nSyntax: with expression as variable: body".to_string(),
            "lambda" => "lambda keyword - Anonymous function\n\nSyntax: lambda args: expression".to_string(),
            "list" => "list - Mutable sequence type\n\nMethods: append(), extend(), insert(), remove()".to_string(),
            "dict" => "dict - Mapping type (dictionary)\n\nMethods: keys(), values(), items(), get()".to_string(),
            "tuple" => "tuple - Immutable sequence type\n\nUsage: t = (1, 2, 3) or t = 1, 2, 3".to_string(),
            "set" => "set - Unordered collection of unique elements\n\nMethods: add(), remove(), union(), intersection()".to_string(),
            "str" => "str - String type\n\nMethods: split(), join(), replace(), strip(), format()".to_string(),
            "int" => "int - Integer type\n\nUsage: int(value) or int(string, base)".to_string(),
            "float" => "float - Floating point number\n\nUsage: float(value) or float(string)".to_string(),
            "bool" => "bool - Boolean type\n\nValues: True, False".to_string(),
            "len" => "len() - Return length of object\n\nUsage: len(sequence)".to_string(),
            "range" => "range() - Generate sequence of numbers\n\nUsage: range(start, stop, step)".to_string(),
            "enumerate" => "enumerate() - Add counter to iterable\n\nUsage: for i, item in enumerate(seq):".to_string(),
            "zip" => "zip() - Combine multiple iterables\n\nUsage: for a, b in zip(seq1, seq2):".to_string(),
            "map" => "map() - Apply function to iterable\n\nUsage: map(function, iterable)".to_string(),
            "filter" => "filter() - Filter iterable with function\n\nUsage: filter(function, iterable)".to_string(),
            "sorted" => "sorted() - Return sorted list\n\nUsage: sorted(iterable, key=None, reverse=False)".to_string(),
            "print" => "print() - Output to stdout\n\nUsage: print(value, sep=' ', end='\\n')".to_string(),
            "open" => "open() - Open file\n\nUsage: open(filename, mode='r', encoding=None)".to_string(),
            "isinstance" => "isinstance() - Check object type\n\nUsage: isinstance(obj, class_or_tuple)".to_string(),
            "hasattr" => "hasattr() - Check if object has attribute\n\nUsage: hasattr(obj, 'attribute')".to_string(),
            "__init__" => "__init__() - Constructor method\n\nUsage: def __init__(self, params): pass".to_string(),
            "__str__" => "__str__() - String representation\n\nUsage: def __str__(self): return string".to_string(),
            "__repr__" => "__repr__() - Developer string representation\n\nUsage: def __repr__(self): return string".to_string(),
            "async" => "async keyword - Define asynchronous function\n\nUsage: async def func(): await expression".to_string(),
            "await" => "await keyword - Wait for coroutine\n\nUsage: result = await coroutine()".to_string(),
            _ => format!("{} - Python keyword/identifier", keyword),
        }
    }
}

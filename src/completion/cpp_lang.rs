// C++ language completion provider

use super::LanguageProvider;

pub struct CppProvider;

impl LanguageProvider for CppProvider {
    fn keywords(&self) -> &[&'static str] {
        &[
            "alignas", "alignof", "and", "and_eq", "asm", "atomic_cancel", "atomic_commit",
            "atomic_noexcept", "auto", "bitand", "bitor", "bool", "break", "case", "catch",
            "char", "char8_t", "char16_t", "char32_t", "class", "compl", "concept", "const",
            "consteval", "constexpr", "constinit", "const_cast", "continue", "co_await",
            "co_return", "co_yield", "decltype", "default", "delete", "do", "double",
            "dynamic_cast", "else", "enum", "explicit", "export", "extern", "false", "float",
            "for", "friend", "goto", "if", "inline", "int", "long", "mutable", "namespace",
            "new", "noexcept", "not", "not_eq", "nullptr", "operator", "or", "or_eq",
            "private", "protected", "public", "reflexpr", "register", "reinterpret_cast",
            "requires", "return", "short", "signed", "sizeof", "static", "static_assert",
            "static_cast", "struct", "switch", "synchronized", "template", "this", "thread_local",
            "throw", "true", "try", "typedef", "typeid", "typename", "union", "unsigned",
            "using", "virtual", "void", "volatile", "wchar_t", "while", "xor", "xor_eq",
            "std", "cout", "cin", "endl", "vector", "string", "map", "set", "pair", "make_pair",
            "unique_ptr", "shared_ptr", "weak_ptr", "make_unique", "make_shared"
        ]
    }

    fn snippets(&self) -> &[(&'static str, &'static str)] {
        &[
            ("main", "int main() {\n    ${1:// code}\n    return 0;\n}"),
            ("class", "class ${1:ClassName} {\npublic:\n    ${2:// public members}\nprivate:\n    ${3:// private members}\n};"),
            ("function", "${1:int} ${2:function_name}(${3:parameters}) {\n    ${4:// body}\n    return ${5:0};\n}"),
            ("template", "template<typename ${1:T}>\n${2:void} ${3:function_name}(${4:T param}) {\n    ${5:// body}\n}"),
            ("namespace", "namespace ${1:name} {\n    ${2:// contents}\n}"),
            ("if", "if (${1:condition}) {\n    ${2:// body}\n}"),
            ("for", "for (${1:int i = 0}; ${2:i < n}; ${3:++i}) {\n    ${4:// body}\n}"),
            ("while", "while (${1:condition}) {\n    ${2:// body}\n}"),
            ("try", "try {\n    ${1:// code}\n} catch (const ${2:std::exception}& ${3:e}) {\n    ${4:// handle exception}\n}"),
            ("switch", "switch (${1:expression}) {\n    case ${2:value}:\n        ${3:// code}\n        break;\n    default:\n        ${4:// default}\n        break;\n}"),
            ("include", "#include <${1:iostream}>"),
            ("cout", "std::cout << ${1:\"Hello\"} << std::endl;"),
            ("cin", "std::cin >> ${1:variable};"),
            ("vector", "std::vector<${1:int}> ${2:vec};"),
            ("unique_ptr", "std::unique_ptr<${1:Type}> ${2:ptr} = std::make_unique<${1:Type}>(${3:args});"),
        ]
    }

    fn get_documentation(&self, keyword: &str) -> String {
        match keyword {
            "std" => "std namespace - Standard library namespace\n\nContains: vector, string, map, cout, cin, etc.".to_string(),
            "cout" => "std::cout - Standard output stream\n\nUsage: std::cout << value << std::endl;".to_string(),
            "cin" => "std::cin - Standard input stream\n\nUsage: std::cin >> variable;".to_string(),
            "endl" => "std::endl - End line and flush\n\nUsage: std::cout << \"text\" << std::endl;".to_string(),
            "vector" => "std::vector - Dynamic array\n\nMethods: push_back(), pop_back(), size(), clear()".to_string(),
            "string" => "std::string - String class\n\nMethods: length(), substr(), find(), replace()".to_string(),
            "map" => "std::map - Associative container\n\nMethods: insert(), find(), erase(), size()".to_string(),
            "set" => "std::set - Ordered set container\n\nMethods: insert(), find(), erase(), size()".to_string(),
            "pair" => "std::pair - Two-element container\n\nUsage: std::pair<int, string> p;".to_string(),
            "make_pair" => "std::make_pair - Create pair\n\nUsage: auto p = std::make_pair(1, \"hello\");".to_string(),
            "unique_ptr" => "std::unique_ptr - Exclusive ownership smart pointer\n\nUsage: auto ptr = std::make_unique<Type>(args);".to_string(),
            "shared_ptr" => "std::shared_ptr - Shared ownership smart pointer\n\nUsage: auto ptr = std::make_shared<Type>(args);".to_string(),
            "weak_ptr" => "std::weak_ptr - Non-owning smart pointer\n\nUsage: std::weak_ptr<Type> weak = shared;".to_string(),
            "make_unique" => "std::make_unique - Create unique_ptr\n\nUsage: auto ptr = std::make_unique<Type>(args);".to_string(),
            "make_shared" => "std::make_shared - Create shared_ptr\n\nUsage: auto ptr = std::make_shared<Type>(args);".to_string(),
            "class" => "class keyword - Define a class\n\nSyntax: class Name { public: private: };".to_string(),
            "struct" => "struct keyword - Define structure (public by default)\n\nSyntax: struct Name { members; };".to_string(),
            "template" => "template keyword - Generic programming\n\nSyntax: template<typename T> void func(T param);".to_string(),
            "namespace" => "namespace keyword - Define namespace\n\nSyntax: namespace name { contents }".to_string(),
            "using" => "using keyword - Namespace/type alias\n\nSyntax: using namespace std; or using Type = OtherType;".to_string(),
            "public" => "public access specifier - Accessible from anywhere\n\nUsage: public: members".to_string(),
            "private" => "private access specifier - Accessible only within class\n\nUsage: private: members".to_string(),
            "protected" => "protected access specifier - Accessible within class and derived classes\n\nUsage: protected: members".to_string(),
            "virtual" => "virtual keyword - Enable polymorphism\n\nUsage: virtual void func();".to_string(),
            "override" => "override specifier - Mark overridden virtual function\n\nUsage: void func() override;".to_string(),
            "const" => "const qualifier - Immutable data\n\nUsage: const int x = 10; or void func() const;".to_string(),
            "constexpr" => "constexpr keyword - Compile-time constant expression\n\nUsage: constexpr int x = 10;".to_string(),
            "auto" => "auto keyword - Automatic type deduction\n\nUsage: auto var = expression;".to_string(),
            "decltype" => "decltype keyword - Deduce type of expression\n\nUsage: decltype(expr) var;".to_string(),
            "nullptr" => "nullptr - Null pointer literal\n\nUsage: ptr = nullptr;".to_string(),
            "new" => "new operator - Dynamic memory allocation\n\nUsage: Type* ptr = new Type(args);".to_string(),
            "delete" => "delete operator - Dynamic memory deallocation\n\nUsage: delete ptr;".to_string(),
            "try" => "try statement - Exception handling\n\nSyntax: try { code } catch (exception) { handler }".to_string(),
            "catch" => "catch clause - Handle exceptions\n\nSyntax: catch (const std::exception& e) { handler }".to_string(),
            "throw" => "throw statement - Throw exception\n\nSyntax: throw std::runtime_error(\"message\");".to_string(),
            "static_cast" => "static_cast - Safe type conversion\n\nUsage: static_cast<TargetType>(expression)".to_string(),
            "dynamic_cast" => "dynamic_cast - Runtime type conversion\n\nUsage: dynamic_cast<TargetType*>(ptr)".to_string(),
            "const_cast" => "const_cast - Remove const qualifier\n\nUsage: const_cast<Type*>(const_ptr)".to_string(),
            "reinterpret_cast" => "reinterpret_cast - Low-level type conversion\n\nUsage: reinterpret_cast<TargetType>(expression)".to_string(),
            "sizeof" => "sizeof operator - Get size of type/object\n\nUsage: sizeof(Type) or sizeof(object)".to_string(),
            "typeid" => "typeid operator - Runtime type information\n\nUsage: typeid(object).name()".to_string(),
            "friend" => "friend keyword - Grant access to private members\n\nUsage: friend class FriendClass;".to_string(),
            "operator" => "operator keyword - Operator overloading\n\nUsage: Type operator+(const Type& other);".to_string(),
            "explicit" => "explicit keyword - Prevent implicit conversions\n\nUsage: explicit constructor(args);".to_string(),
            "mutable" => "mutable keyword - Allow modification in const methods\n\nUsage: mutable int cache;".to_string(),
            "inline" => "inline keyword - Suggest function inlining\n\nUsage: inline int func() { return value; }".to_string(),
            "extern" => "extern keyword - External linkage\n\nUsage: extern \"C\" { declarations }".to_string(),
            "static" => "static keyword - Internal linkage or class member\n\nUsage: static int var; or static void func();".to_string(),
            _ => format!("{} - C++ language element for object-oriented and system programming", keyword),
        }
    }
}

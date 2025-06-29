// C language completion provider

use super::LanguageProvider;

pub struct CProvider;

impl LanguageProvider for CProvider {
    fn keywords(&self) -> &[&'static str] {
        &[
            "auto", "break", "case", "char", "const", "continue", "default", "do", "double", "else",
            "enum", "extern", "float", "for", "goto", "if", "inline", "int", "long", "register",
            "restrict", "return", "short", "signed", "sizeof", "static", "struct", "switch",
            "typedef", "union", "unsigned", "void", "volatile", "while", "_Alignas", "_Alignof",
            "_Atomic", "_Static_assert", "_Noreturn", "_Thread_local", "_Generic", "printf",
            "scanf", "malloc", "free", "sizeof", "NULL", "true", "false", "bool", "size_t",
            "FILE", "stdin", "stdout", "stderr", "fopen", "fclose", "fread", "fwrite", "fprintf"
        ]
    }

    fn snippets(&self) -> &[(&'static str, &'static str)] {
        &[
            ("main", "int main(int argc, char *argv[]) {\n    ${1:// code}\n    return 0;\n}"),
            ("function", "${1:int} ${2:function_name}(${3:parameters}) {\n    ${4:// body}\n    return ${5:0};\n}"),
            ("struct", "struct ${1:name} {\n    ${2:int field};\n};"),
            ("typedef", "typedef struct {\n    ${2:int field};\n} ${1:TypeName};"),
            ("if", "if (${1:condition}) {\n    ${2:// body}\n}"),
            ("for", "for (${1:int i = 0}; ${2:i < n}; ${3:i++}) {\n    ${4:// body}\n}"),
            ("while", "while (${1:condition}) {\n    ${2:// body}\n}"),
            ("switch", "switch (${1:expression}) {\n    case ${2:value}:\n        ${3:// code}\n        break;\n    default:\n        ${4:// default}\n        break;\n}"),
            ("include", "#include <${1:stdio.h}>"),
            ("define", "#define ${1:NAME} ${2:value}"),
            ("printf", "printf(\"${1:%d}\\n\", ${2:variable});"),
            ("scanf", "scanf(\"${1:%d}\", &${2:variable});"),
            ("malloc", "${1:int} *${2:ptr} = (${1:int}*)malloc(${3:size} * sizeof(${1:int}));"),
            ("free", "free(${1:ptr});\n${1:ptr} = NULL;"),
            ("enum", "enum ${1:name} {\n    ${2:VALUE1},\n    ${3:VALUE2}\n};"),
        ]
    }

    fn get_documentation(&self, keyword: &str) -> String {
        match keyword {
            "printf" => "printf() - Print formatted output\n\nSyntax: printf(\"format\", args...)".to_string(),
            "scanf" => "scanf() - Read formatted input\n\nSyntax: scanf(\"format\", &variables...)".to_string(),
            "malloc" => "malloc() - Allocate memory\n\nSyntax: void* malloc(size_t size)".to_string(),
            "free" => "free() - Deallocate memory\n\nSyntax: free(ptr)".to_string(),
            "sizeof" => "sizeof - Get size of type/variable\n\nSyntax: sizeof(type) or sizeof(variable)".to_string(),
            "struct" => "struct keyword - Define structure\n\nSyntax: struct Name { type field; };".to_string(),
            "typedef" => "typedef keyword - Create type alias\n\nSyntax: typedef existing_type new_name;".to_string(),
            "enum" => "enum keyword - Define enumeration\n\nSyntax: enum Name { VALUE1, VALUE2 };".to_string(),
            "union" => "union keyword - Define union\n\nSyntax: union Name { type field1; type field2; };".to_string(),
            "if" => "if statement - Conditional execution\n\nSyntax: if (condition) { body }".to_string(),
            "for" => "for loop - Iterate with counter\n\nSyntax: for (init; condition; increment) { body }".to_string(),
            "while" => "while loop - Execute while condition is true\n\nSyntax: while (condition) { body }".to_string(),
            "do" => "do-while loop - Execute at least once\n\nSyntax: do { body } while (condition);".to_string(),
            "switch" => "switch statement - Multi-way branching\n\nSyntax: switch (expr) { case value: break; }".to_string(),
            "case" => "case label - Switch case\n\nSyntax: case value: statements; break;".to_string(),
            "break" => "break statement - Exit loop or switch\n\nUsage: break;".to_string(),
            "continue" => "continue statement - Skip iteration\n\nUsage: continue;".to_string(),
            "return" => "return statement - Return from function\n\nSyntax: return value;".to_string(),
            "goto" => "goto statement - Jump to label\n\nSyntax: goto label;".to_string(),
            "const" => "const qualifier - Immutable data\n\nUsage: const int x = 10;".to_string(),
            "static" => "static keyword - Internal linkage or persistent storage\n\nUsage: static int var;".to_string(),
            "extern" => "extern keyword - External linkage\n\nUsage: extern int global_var;".to_string(),
            "volatile" => "volatile qualifier - Prevents optimization\n\nUsage: volatile int flag;".to_string(),
            "auto" => "auto keyword - Automatic storage class\n\nUsage: auto int var; (rarely used)".to_string(),
            "register" => "register keyword - Suggest register storage\n\nUsage: register int i;".to_string(),
            "inline" => "inline keyword - Suggest function inlining\n\nUsage: inline int func() { }".to_string(),
            "restrict" => "restrict qualifier - Pointer aliasing hint\n\nUsage: int *restrict ptr;".to_string(),
            "NULL" => "NULL - Null pointer constant\n\nUsage: ptr = NULL;".to_string(),
            "FILE" => "FILE - File handle type\n\nUsage: FILE *fp = fopen(\"file.txt\", \"r\");".to_string(),
            "stdin" => "stdin - Standard input stream\n\nUsage: fgets(buffer, size, stdin);".to_string(),
            "stdout" => "stdout - Standard output stream\n\nUsage: fprintf(stdout, \"text\");".to_string(),
            "stderr" => "stderr - Standard error stream\n\nUsage: fprintf(stderr, \"error\");".to_string(),
            "fopen" => "fopen() - Open file\n\nSyntax: FILE* fopen(const char* filename, const char* mode)".to_string(),
            "fclose" => "fclose() - Close file\n\nSyntax: int fclose(FILE* stream)".to_string(),
            "fread" => "fread() - Read from file\n\nSyntax: size_t fread(void* ptr, size_t size, size_t count, FILE* stream)".to_string(),
            "fwrite" => "fwrite() - Write to file\n\nSyntax: size_t fwrite(const void* ptr, size_t size, size_t count, FILE* stream)".to_string(),
            "fprintf" => "fprintf() - Print formatted to file\n\nSyntax: int fprintf(FILE* stream, const char* format, ...)".to_string(),
            _ => format!("{} - C language element for system and application programming", keyword),
        }
    }
}

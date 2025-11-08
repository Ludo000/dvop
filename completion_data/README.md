# Completion Data JSON Format

This directory contains JSON files that define completion data for various programming languages. Each JSON file provides keywords and code snippets for intelligent code completion in the text editor.

## File Structure

Each language has its own JSON file named `{language}.json` (e.g., `rust.json`, `python.json`, `javascript.json`).

## JSON Schema

Each JSON file follows this structure:

```json
{
  "language": "language_name",
  "description": "Human-readable description of the language",
  "keywords": [
    {
      "keyword": "keyword_name",
      "type": "keyword",
      "description": "Detailed explanation of what this keyword does and when to use it",
      "example": "Concrete example showing the keyword in use",
      "category": "logical_grouping_category"
    }
  ],
  "snippets": [
    {
      "trigger": "trigger_text", 
      "description": "What this snippet does and when to use it",
      "content": "Template code with ${1:placeholder} syntax for tab stops",
      "category": "logical_grouping_category"
    }
  ]
}
```

## Field Descriptions

### Root Level Fields

- **language**: Identifier for the programming language (must match the language detection in the editor)
- **description**: Human-readable description shown in UI and help

### Keyword Fields

- **keyword**: The exact keyword or identifier text
- **type**: Always "keyword" for language keywords
- **description**: Detailed explanation of functionality, usage, and best practices
- **example**: Code example showing practical usage
- **category**: Logical grouping (e.g., "control_flow", "variable_declaration", "function_declaration")

### Snippet Fields  

- **trigger**: Text that triggers this snippet (what user types to activate)
- **description**: Explanation of what the snippet does
- **content**: Template code with VSCode-style placeholders:
  - `${1:placeholder_text}` - First tab stop with default text
  - `${2:another_placeholder}` - Second tab stop
  - `${1}` - Reference to first placeholder
- **category**: Logical grouping (e.g., "function", "class", "control_flow", "error_handling")

## Common Categories

### Keywords
- `variable_declaration` - let, var, const, mut
- `control_flow` - if, else, for, while, loop, break, continue
- `function_declaration` - fn, def, function
- `type_declaration` - struct, class, enum, trait, interface
- `module_system` - import, export, use, mod, from
- `error_handling` - try, catch, except, throw, panic
- `async` - async, await, Promise-related keywords
- `visibility` - pub, private, protected, public

### Snippets
- `function` - Function templates and definitions
- `class` - Class and object-oriented templates
- `control_flow` - Conditional and loop templates
- `error_handling` - Try-catch and error handling patterns
- `async` - Asynchronous programming patterns
- `testing` - Unit test and testing templates
- `documentation` - Documentation comment templates
- `import` - Import and module templates

## Adding Your Own Completion Data

### 1. Create a New Language File

Create a new JSON file named after your language:
```bash
touch completion_data/my_language.json
```

### 2. Define Basic Structure

Start with the basic JSON structure:
```json
{
  "language": "my_language",
  "description": "My Programming Language",
  "keywords": [],
  "snippets": []
}
```

### 3. Add Keywords

Add keywords with comprehensive documentation:
```json
{
  "keyword": "my_keyword",
  "type": "keyword", 
  "description": "Detailed explanation of what this keyword does, when to use it, and why it's useful. Include best practices and common pitfalls.",
  "example": "my_keyword variable_name = value; // Shows practical usage",
  "category": "variable_declaration"
}
```

### 4. Add Snippets

Create code templates with placeholders:
```json
{
  "trigger": "my_snippet",
  "description": "What this code template does and when to use it", 
  "content": "my_keyword ${1:name}(${2:parameters}) {\n    ${3:// body}\n    return ${4:result};\n}",
  "category": "function"
}
```

### 5. Test Your Changes

1. Restart the text editor to load new completion data
2. Open a file with your language extension
3. Press `Ctrl+Space` or `F1` to test completion
4. Check that your keywords and snippets appear with proper documentation

## Best Practices

### Writing Good Documentation

1. **Be Educational**: Explain not just what something does, but when and why to use it
2. **Include Context**: Mention related concepts, alternatives, and gotchas
3. **Show Examples**: Provide concrete, practical examples that users can adapt
4. **Use Clear Language**: Write for programmers who may be learning the language

### Organizing Categories

1. **Use Consistent Names**: Stick to the common categories listed above when possible
2. **Group Logically**: Put related concepts together (all variable keywords in one category)
3. **Keep It Simple**: Don't create too many micro-categories

### Creating Useful Snippets

1. **Common Patterns**: Focus on code structures you write frequently
2. **Good Defaults**: Provide sensible placeholder text that guides usage  
3. **Proper Indentation**: Use consistent spacing that matches language conventions
4. **Tab Stop Flow**: Order placeholders in the sequence users would naturally fill them

### JSON Validation

Always validate your JSON before committing:
```bash
# Using jq (recommended)
jq . completion_data/my_language.json

# Using python
python -m json.tool completion_data/my_language.json

# Using node
node -e "JSON.parse(require('fs').readFileSync('completion_data/my_language.json'))"
```

## Examples

See the existing files for comprehensive examples:
- `rust.json` - Complex language with detailed type system documentation
- `python.json` - Dynamic language with emphasis on readability
- `javascript.json` - Web language with modern async patterns
- `typescript.json` - TypeScript-specific features including types, interfaces, and utility types
- `css.json` - CSS properties and selectors
- `html.json` - HTML elements and attributes
- `svelte.json` - Svelte framework components and syntax

## Contributing

1. **Add Missing Languages**: Create JSON files for languages not yet supported
2. **Improve Existing Data**: Add more keywords, better descriptions, useful snippets
3. **Fix Errors**: Correct typos, improve examples, update outdated information
4. **Suggest Categories**: Propose better organization schemes

## Technical Notes

- Files are loaded at startup and cached for performance
- Language detection is based on file extension and buffer language settings
- JSON data takes priority over hardcoded completion providers
- Invalid JSON files will be skipped with warnings
- The system gracefully falls back to basic completion if JSON fails to load

## Troubleshooting

### Completion Not Working
1. Check file naming: must be `{language}.json` 
2. Validate JSON syntax with a validator
3. Check console output for loading errors
4. Verify language detection is working correctly

### Documentation Not Showing
1. Ensure `description` and `example` fields are present and non-empty
2. Check for special characters that might need escaping
3. Verify the completion popup is large enough to display documentation

### Snippets Not Expanding
1. Check placeholder syntax: use `${n:text}` format
2. Ensure content doesn't have invalid escape sequences
3. Test with simple snippets first, then add complexity

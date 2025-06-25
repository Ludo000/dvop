// CSS language completion provider

use super::LanguageProvider;

pub struct CssProvider;

impl LanguageProvider for CssProvider {
    fn keywords(&self) -> &[&'static str] {
        &[
            "color", "background", "background-color", "background-image", "background-repeat",
            "background-position", "background-size", "border", "border-color", "border-style",
            "border-width", "border-radius", "margin", "padding", "width", "height", "min-width",
            "max-width", "min-height", "max-height", "display", "position", "top", "right",
            "bottom", "left", "float", "clear", "overflow", "z-index", "font", "font-family",
            "font-size", "font-weight", "font-style", "text-align", "text-decoration",
            "text-transform", "line-height", "letter-spacing", "word-spacing", "white-space",
            "vertical-align", "list-style", "table-layout", "border-collapse", "border-spacing",
            "caption-side", "empty-cells", "cursor", "outline", "visibility", "opacity",
            "box-shadow", "text-shadow", "transform", "transition", "animation", "flex",
            "grid", "align-items", "justify-content", "flex-direction", "flex-wrap",
            "align-content", "justify-items", "place-items", "gap", "row-gap", "column-gap"
        ]
    }

    fn snippets(&self) -> &[(&'static str, &'static str)] {
        &[
            ("selector", "${1:.class} {\n    ${2:property}: ${3:value};\n}"),
            ("class", ".${1:class-name} {\n    ${2:property}: ${3:value};\n}"),
            ("id", "#${1:id-name} {\n    ${2:property}: ${3:value};\n}"),
            ("element", "${1:element} {\n    ${2:property}: ${3:value};\n}"),
            ("media", "@media (${1:max-width: 768px}) {\n    ${2:/* responsive styles */}\n}"),
            ("keyframes", "@keyframes ${1:animation-name} {\n    0% {\n        ${2:property}: ${3:start-value};\n    }\n    100% {\n        ${4:property}: ${5:end-value};\n    }\n}"),
            ("flexbox", "display: flex;\nflex-direction: ${1:row};\njustify-content: ${2:center};\nalign-items: ${3:center};"),
            ("grid", "display: grid;\ngrid-template-columns: ${1:repeat(auto-fit, minmax(200px, 1fr))};\ngap: ${2:1rem};"),
            ("transition", "transition: ${1:all} ${2:0.3s} ${3:ease};"),
            ("transform", "transform: ${1:translateX(0)} ${2:scale(1)} ${3:rotate(0deg)};"),
            ("box-shadow", "box-shadow: ${1:0 2px 4px} rgba(0, 0, 0, ${2:0.1});"),
            ("text-shadow", "text-shadow: ${1:1px 1px 2px} rgba(0, 0, 0, ${2:0.5});"),
            ("gradient", "background: linear-gradient(${1:45deg}, ${2:#ff0000}, ${3:#0000ff});"),
            ("border-radius", "border-radius: ${1:8px};"),
            ("font", "font: ${1:normal} ${2:16px}/${3:1.5} ${4:'Arial, sans-serif'};"),
            ("reset", "* {\n    margin: 0;\n    padding: 0;\n    box-sizing: border-box;\n}"),
        ]
    }

    fn get_documentation(&self, keyword: &str) -> String {
        match keyword {
            "display" => "display property - Controls element layout\n\nValues: block, inline, flex, grid, none".to_string(),
            "position" => "position property - Element positioning\n\nValues: static, relative, absolute, fixed, sticky".to_string(),
            "flex" => "flex property - Flexible box layout\n\nShorthand for: flex-grow flex-shrink flex-basis".to_string(),
            "grid" => "grid property - Grid layout system\n\nUsage: display: grid; grid-template-columns: ...".to_string(),
            "color" => "color property - Text color\n\nValues: #hex, rgb(), rgba(), hsl(), named colors".to_string(),
            "background" => "background shorthand - Background properties\n\nValues: color image repeat position size".to_string(),
            "background-color" => "background-color property - Background color\n\nValues: #hex, rgb(), rgba(), hsl(), named colors".to_string(),
            "background-image" => "background-image property - Background image\n\nValues: url('image.jpg'), linear-gradient(), none".to_string(),
            "background-repeat" => "background-repeat property - Image repetition\n\nValues: repeat, no-repeat, repeat-x, repeat-y".to_string(),
            "background-position" => "background-position property - Image position\n\nValues: top, center, bottom, left, right, %".to_string(),
            "background-size" => "background-size property - Image size\n\nValues: auto, cover, contain, px, %".to_string(),
            "border" => "border shorthand - Border properties\n\nSyntax: width style color".to_string(),
            "border-radius" => "border-radius property - Rounded corners\n\nValues: px, %, em (1-4 values)".to_string(),
            "margin" => "margin property - Outer spacing\n\nValues: px, %, em, auto (1-4 values)".to_string(),
            "padding" => "padding property - Inner spacing\n\nValues: px, %, em (1-4 values)".to_string(),
            "width" => "width property - Element width\n\nValues: px, %, em, auto, vw".to_string(),
            "height" => "height property - Element height\n\nValues: px, %, em, auto, vh".to_string(),
            "min-width" => "min-width property - Minimum width\n\nValues: px, %, em".to_string(),
            "max-width" => "max-width property - Maximum width\n\nValues: px, %, em, none".to_string(),
            "min-height" => "min-height property - Minimum height\n\nValues: px, %, em".to_string(),
            "max-height" => "max-height property - Maximum height\n\nValues: px, %, em, none".to_string(),
            "font-family" => "font-family property - Font typeface\n\nValues: 'font name', serif, sans-serif, monospace".to_string(),
            "font-size" => "font-size property - Text size\n\nValues: px, em, rem, %, keywords (small, large)".to_string(),
            "font-weight" => "font-weight property - Text weight\n\nValues: normal, bold, 100-900".to_string(),
            "font-style" => "font-style property - Text style\n\nValues: normal, italic, oblique".to_string(),
            "text-align" => "text-align property - Text alignment\n\nValues: left, center, right, justify".to_string(),
            "text-decoration" => "text-decoration property - Text decoration\n\nValues: none, underline, overline, line-through".to_string(),
            "text-transform" => "text-transform property - Text case\n\nValues: none, uppercase, lowercase, capitalize".to_string(),
            "line-height" => "line-height property - Line spacing\n\nValues: normal, number, px, em, %".to_string(),
            "letter-spacing" => "letter-spacing property - Character spacing\n\nValues: normal, px, em".to_string(),
            "word-spacing" => "word-spacing property - Word spacing\n\nValues: normal, px, em".to_string(),
            "white-space" => "white-space property - Whitespace handling\n\nValues: normal, nowrap, pre, pre-wrap".to_string(),
            "vertical-align" => "vertical-align property - Vertical alignment\n\nValues: baseline, top, middle, bottom, px, %".to_string(),
            "list-style" => "list-style shorthand - List styling\n\nValues: type position image".to_string(),
            "cursor" => "cursor property - Mouse cursor\n\nValues: auto, pointer, text, move, not-allowed".to_string(),
            "outline" => "outline property - Element outline\n\nSyntax: width style color".to_string(),
            "visibility" => "visibility property - Element visibility\n\nValues: visible, hidden, collapse".to_string(),
            "opacity" => "opacity property - Element transparency\n\nValues: 0.0 to 1.0".to_string(),
            "overflow" => "overflow property - Content overflow\n\nValues: visible, hidden, scroll, auto".to_string(),
            "z-index" => "z-index property - Stacking order\n\nValues: auto, integer".to_string(),
            "top" => "top property - Positioned element offset\n\nValues: auto, px, %, em".to_string(),
            "right" => "right property - Positioned element offset\n\nValues: auto, px, %, em".to_string(),
            "bottom" => "bottom property - Positioned element offset\n\nValues: auto, px, %, em".to_string(),
            "left" => "left property - Positioned element offset\n\nValues: auto, px, %, em".to_string(),
            "float" => "float property - Element floating\n\nValues: none, left, right".to_string(),
            "clear" => "clear property - Clear floating\n\nValues: none, left, right, both".to_string(),
            "box-shadow" => "box-shadow property - Element shadow\n\nSyntax: h-offset v-offset blur spread color".to_string(),
            "text-shadow" => "text-shadow property - Text shadow\n\nSyntax: h-offset v-offset blur color".to_string(),
            "transform" => "transform property - Element transformation\n\nValues: translate(), rotate(), scale(), skew()".to_string(),
            "transition" => "transition property - Animated changes\n\nSyntax: property duration timing-function delay".to_string(),
            "animation" => "animation property - Keyframe animations\n\nSyntax: name duration timing-function delay iteration-count".to_string(),
            "flex-direction" => "flex-direction property - Flex container direction\n\nValues: row, column, row-reverse, column-reverse".to_string(),
            "flex-wrap" => "flex-wrap property - Flex item wrapping\n\nValues: nowrap, wrap, wrap-reverse".to_string(),
            "justify-content" => "justify-content property - Main axis alignment\n\nValues: flex-start, center, flex-end, space-between, space-around".to_string(),
            "align-items" => "align-items property - Cross axis alignment\n\nValues: stretch, flex-start, center, flex-end, baseline".to_string(),
            "align-content" => "align-content property - Multi-line alignment\n\nValues: stretch, flex-start, center, flex-end, space-between".to_string(),
            "gap" => "gap property - Grid/flex gap\n\nValues: px, em, rem, %".to_string(),
            "row-gap" => "row-gap property - Row spacing\n\nValues: px, em, rem, %".to_string(),
            "column-gap" => "column-gap property - Column spacing\n\nValues: px, em, rem, %".to_string(),
            "justify-items" => "justify-items property - Grid item horizontal alignment\n\nValues: start, center, end, stretch".to_string(),
            "place-items" => "place-items property - Grid item alignment shorthand\n\nValues: align-items justify-items".to_string(),
            "table-layout" => "table-layout property - Table layout algorithm\n\nValues: auto, fixed".to_string(),
            "border-collapse" => "border-collapse property - Table border model\n\nValues: separate, collapse".to_string(),
            "border-spacing" => "border-spacing property - Table border spacing\n\nValues: px, em".to_string(),
            "caption-side" => "caption-side property - Table caption position\n\nValues: top, bottom".to_string(),
            "empty-cells" => "empty-cells property - Empty cell borders\n\nValues: show, hide".to_string(),
            _ => format!("{} - CSS property", keyword),
        }
    }
}

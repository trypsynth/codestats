use phf::{Map, phf_map};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Language {
{%- for field_spec in struct_fields %}
	pub {{ field_spec.0 }}: {{ field_spec.1 }},
{%- endfor %}
}

pub static LANGUAGES: &[Language] = &[
{%- for lang in languages %}
	Language {
{%- for field_spec in struct_fields %}
		{{ field_spec.0 }}: {{ lang[field_spec.0] | field_render(field_type=field_spec.1) }},
{%- endfor %}
	},
{%- endfor %}
];

pub static LANGUAGE_MAP: Map<&'static str, &Language> = phf_map! {
{%- for lang in languages %}
	{{ lang.name | rust_string }} => &LANGUAGES[{{ loop.index0 }}],
{%- endfor %}
};

pub static PATTERN_MAP: Map<&'static str, &Language> = phf_map! {
{%- for mapping in pattern_mappings %}
	{{ mapping.0 | rust_string }} => &LANGUAGES[{{ mapping.1 }}],
{%- endfor %}
};

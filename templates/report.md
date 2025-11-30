# Codestats for `{{ title }}`

## Summary

- Files: {{ total_files }}
- Lines: {{ total_lines }}
- Size: {{ total_size_human }}
{% if !line_breakdown.is_empty() -%}
- Line types: {{ line_breakdown | join(", ") }}
{% endif -%}
{% if !totals.is_empty() -%}
- Totals: {{ totals | join(", ") }}
{% endif -%}

{% if languages.is_empty() -%}
_No recognized programming languages found._
{% else -%}

## Languages

| Language | Files | Lines | Code % | Comment % | Blank % | Shebang % | Size |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |

{% for lang in languages -%}
| {{ lang.name | md_escape }} | {{ lang.files }} | {{ lang.lines }} | {{ lang.code_percentage }}% | {{ lang.comment_percentage }}% | {{ lang.blank_percentage }}% | {{ lang.shebang_percentage }}% | {{ lang.size_human | md_escape }} |
{% endfor -%}

{% if show_files -%}

## Files

{% for lang in languages -%}
{% match lang.files_detail -%}
{% when Some with (files) -%}

### {{ lang.name | md_escape }}

| File | Total lines | Code lines | Comment lines | Blank lines | Shebang lines | Size |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |

{% for file in files -%}
| {{ file.path | md_escape }} | {{ file.total_lines }} | {{ file.code_lines }} | {{ file.comment_lines }} | {{ file.blank_lines }} | {{ file.shebang_lines }} | {{ file.size_human | md_escape }} |
{% endfor -%}

{% when None -%}{% endmatch -%}
{% endfor -%}
{% endif -%}
{% endif -%}

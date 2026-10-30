# Codestats for `{{ title }}`

## Summary

- Files: {{ summary.total_files | fmt_number(ctx) }}
- Lines: {{ summary.total_lines | fmt_number(ctx) }}
- Size: {{ summary.total_size_human }}
{% if !line_breakdown.is_empty() -%}
- Line types: {{ line_breakdown | join(", ") }}
{% endif -%}
{% if !totals.is_empty() -%}
- Totals: {{ totals | join(", ") }}
{% endif -%}

{%- macro file_table(files) -%}
| File | Total lines | Code lines | Comment lines | Blank lines | Shebang lines | Size |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |

{% for file in files -%}
| {{ file.path | md_escape }} | {{ file.total_lines | fmt_number(ctx) }} | {{ file.code_lines | fmt_number(ctx) }} | {{ file.comment_lines | fmt_number(ctx) }} | {{ file.blank_lines | fmt_number(ctx) }} | {{ file.shebang_lines | fmt_number(ctx) }} | {{ file.size_human | md_escape }} |
{% endfor -%}
{%- endmacro -%}

{% if by_dir -%}
{% if directories.is_empty() -%}
_No recognized programming languages found._
{% else -%}

## Directories

| Directory | Files | Lines | Code % | Comment % | Blank % | Shebang % | Size |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |

{% for dir in directories -%}
| {{ dir.path | md_escape }} | {{ dir.files | fmt_number(ctx) }} | {{ dir.lines | fmt_number(ctx) }} | {{ dir.code_percentage | fmt_percent(ctx) }}% | {{ dir.comment_percentage | fmt_percent(ctx) }}% | {{ dir.blank_percentage | fmt_percent(ctx) }}% | {{ dir.shebang_percentage | fmt_percent(ctx) }}% | {{ dir.size_human | md_escape }} |
{% endfor -%}

{% if show_files -%}

## Files

{% for dir in directories -%}
{% match dir.files_detail -%}
{% when Some with (files) -%}

### {{ dir.path | md_escape }}

{% call file_table(files) %}{% endcall -%}

{% when None -%}{% endmatch -%}
{% endfor -%}
{% endif -%}
{% endif -%}
{% else -%}
{% if languages.is_empty() -%}
_No recognized programming languages found._
{% else -%}

## Languages

| Language | Files | Lines | Average Lines per File | Code % | Comment % | Blank % | Shebang % | Size |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |

{% for lang in languages -%}
| {{ lang.name | md_escape }} | {{ lang.files | fmt_number(ctx) }} | {{ lang.lines | fmt_number(ctx) }} | {{ lang.avg_lines_per_file | fmt_float(1) }} | {{ lang.code_percentage | fmt_percent(ctx) }}% | {{ lang.comment_percentage | fmt_percent(ctx) }}% | {{ lang.blank_percentage | fmt_percent(ctx) }}% | {{ lang.shebang_percentage | fmt_percent(ctx) }}% | {{ lang.size_human | md_escape }} |
{% endfor -%}

{% if show_files -%}

## Files

{% for lang in languages -%}
{% match lang.files_detail -%}
{% when Some with (files) -%}

### {{ lang.name | md_escape }}

{% call file_table(files) %}{% endcall -%}

{% when None -%}{% endmatch -%}
{% endfor -%}
{% endif -%}
{% endif -%}
{% endif -%}

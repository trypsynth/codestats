{#- Each tag sits on its own line and trims the newline before it, so the output has exactly the blank lines written here. -#}
# Codestats for `{{ title }}`

## Summary

- Files: {{ summary.total_files | fmt_number(ctx) }}
- Lines: {{ summary.counts.total | fmt_number(ctx) }}
- Size: {{ summary.total_size_human }}
{%- if !line_breakdown.is_empty() %}
- Line types: {{ line_breakdown | join(", ") }}
{%- endif %}
{%- if !totals.is_empty() %}
- Totals: {{ totals | join(", ") }}
{%- endif %}
{%- macro file_table(files) %}
| File | Total lines | Code lines | Comment lines | Blank lines | Shebang lines | Size |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
{%- for file in files %}
| {{ file.path | md_escape }} | {{ file.counts.total | fmt_number(ctx) }} | {{ file.counts.code | fmt_number(ctx) }} | {{ file.counts.comment | fmt_number(ctx) }} | {{ file.counts.blank | fmt_number(ctx) }} | {{ file.counts.shebang | fmt_number(ctx) }} | {{ file.size_human | md_escape }} |
{%- endfor %}
{%- endmacro %}
{%- if !show_breakdown %}
{%- else if by_dir %}
{%- if directories.is_empty() %}

_No recognized programming languages found._
{%- else %}

## Directories

| Directory | Files | Lines | Code % | Comment % | Blank % | Shebang % | Size |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
{%- for dir in directories %}
| {{ dir.path | md_escape }} | {{ dir.files | fmt_number(ctx) }} | {{ dir.counts.total | fmt_number(ctx) }} | {{ dir.code_percentage | fmt_percent(ctx) }}% | {{ dir.comment_percentage | fmt_percent(ctx) }}% | {{ dir.blank_percentage | fmt_percent(ctx) }}% | {{ dir.shebang_percentage | fmt_percent(ctx) }}% | {{ dir.size_human | md_escape }} |
{%- endfor %}
{%- if show_files %}

## Files
{%- for dir in directories %}
{%- if let Some(files) = dir.files_detail %}

### {{ dir.path | md_escape }}
{% call file_table(files) %}{% endcall %}
{%- endif %}
{%- endfor %}
{%- endif %}
{%- endif %}
{%- else %}
{%- if languages.is_empty() %}

_No recognized programming languages found._
{%- else %}

## Languages

| Language | Files | Lines | Average Lines per File | Code % | Comment % | Blank % | Shebang % | Size |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
{%- for lang in languages %}
| {{ lang.name | md_escape }} | {{ lang.files | fmt_number(ctx) }} | {{ lang.counts.total | fmt_number(ctx) }} | {{ lang.avg_lines_per_file | fmt_float(1) }} | {{ lang.code_percentage | fmt_percent(ctx) }}% | {{ lang.comment_percentage | fmt_percent(ctx) }}% | {{ lang.blank_percentage | fmt_percent(ctx) }}% | {{ lang.shebang_percentage | fmt_percent(ctx) }}% | {{ lang.size_human | md_escape }} |
{%- endfor %}
{%- if show_files %}

## Files
{%- for lang in languages %}
{%- if let Some(files) = lang.files_detail %}

### {{ lang.name | md_escape }}
{% call file_table(files) %}{% endcall %}
{%- endif %}
{%- endfor %}
{%- endif %}
{%- endif %}
{%- endif %}

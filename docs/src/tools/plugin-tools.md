# Plugin Tools

Define custom tools in your config file (`~/.config/clawzero/config.toml` or `clawzero.toml`). Plugin tools extend the agent's capabilities without code changes.

## HTTP plugin

Make HTTP requests with template substitution:

```toml
[[tools]]
name = "weather"
description = "Get current weather for a city"
type = "http"
url = "https://api.weather.example/v1/current?city={{city}}"
method = "GET"

[tools.input_schema.properties.city]
type = "string"
description = "City name"

[tools.input_schema]
required = ["city"]
```

## Bash plugin

Execute bash commands with template substitution:

```toml
[[tools]]
name = "deploy"
description = "Deploy to staging"
type = "bash"
command = "cd {{project_dir}} && make deploy-staging"

[tools.input_schema.properties.project_dir]
type = "string"
description = "Project directory path"
```

## Template substitution

Parameters defined in `input_schema` are substituted into `url` (HTTP) or `command` (bash) using `{{parameter_name}}` syntax.

## Input schema

The `input_schema` follows JSON Schema format:

```toml
[tools.input_schema.properties.param_name]
type = "string"        # "string", "integer", "number", "boolean"
description = "..."

[tools.input_schema]
required = ["param_name"]
```

## Plugin types

| Type | Fields | Description |
|------|--------|-------------|
| `http` | `url`, `method` | Make an HTTP request, return the response body |
| `bash` | `command` | Execute a bash command, return stdout/stderr |

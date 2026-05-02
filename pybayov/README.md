# pybayov

Internal Python utility package. Available via `PYTHONPATH` set in `.bashrc` /
`config.fish`; modules are invoked through shell aliases defined there.

## Modules

| Module                           | Alias                         | Purpose                                                                     |
| -------------------------------- | ----------------------------- | --------------------------------------------------------------------------- |
| `format.py`                      | `bayov-format`                | Run code formatters                                                         |
| `test.py`                        | `bayov-test`                  | Run tests                                                                   |
| `spinner.py`                     | `spinner`                     | Show a terminal spinner                                                     |
| `echo_box.py`                    | `echo-box`                    | Print text in a box                                                         |
| `quick_prompt.py`                | `quick-prompt`                | One-shot prompt to a configured AI model                                    |
| `docker_sandbox.py`              | `docker-sandbox`              | Run a command in a Docker sandbox                                           |
| `generate_commit_description.py` | `generate-commit-description` | Generate commit message via AI                                              |
| `util.py`                        | —                             | Shared ANSI colors and path helpers used by other modules and `dev/link.py` |

## Running directly

```sh
uv run python -m pybayov.spinner
```

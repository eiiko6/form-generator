# Form Generator

A lightweight application for generating dynamic form websites and storing user responses. The form fields, labels, and behavior are fully configurable via a TOML configuration file.

## Usage

The server will listen on `127.0.0.1:8081` by default. You can override with:

```bash
SERVER_PORT=8082 form-generator
```

## Configuration

The application is configured via a TOML file (default: `config.toml`).

See [config.toml](./config.toml) for an example.

### Configuration Fields

* `json_output`: Path to the JSON file where responses are stored.

* `submit_button`: Text displayed on the form’s submit button.

* `fields`: List of form fields
  * `name`: Internal key for storage.
  * `title`: Label displayed in the form.
  * `description`: Short description displayed below the field.
  * `answer_type`: Type of input (`text`, `number`, `email`, `password`, `url`, `tel`, `textarea`).
  * `html_before` (optional): HTML snippet rendered before the field.
  * `html_after` (optional): HTML snippet rendered after the field.

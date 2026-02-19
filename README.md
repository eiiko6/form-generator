# Form Generator

A lightweight application for generating dynamic form websites and storing user responses. The form fields, labels, and behavior are fully configurable via a TOML configuration file.

## Configuration

The application is configured via a TOML file (default: `config.toml`).

See [config.toml](./config.toml) for an example.

### Configuration Fields

* `json_output`: Path to the JSON file where responses are stored.

* `form_title`: Title of the page and form.

* `submit_button`: Text displayed on the form’s submit button.

* `fields`: List of form fields
  * `name`: Internal key for storage.
  * `title`: Label displayed in the form.
  * `description`: Short description displayed below the field.
  * `answer_type`: Type of input (`text`, `number`, `email`, `password`, `url`, `tel`, `textarea`, `select`, `checkbox`, etc).
  * `html_before` (optional): HTML snippet rendered before the field.
  * `html_after` (optional): HTML snippet rendered after the field.

## Lib

This crate also provides a library so you can embed the form into an axum server.  
The `app_router` function provides an axum router with provided routes.

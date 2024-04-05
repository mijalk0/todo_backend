# todo_backend
Rust backend for a todo list, using [axum](https://github.com/tokio-rs/axum) and [sqlx](https://github.com/launchbadge/sqlx) with a [postgres docker container](https://hub.docker.com/_/postgres). 
 
# Preparation

Ensure `rustc`, `cargo`, `docker`, and `docker compose`/`docker-dompose` are installed. Create a `.env` file and set `DATABASE_URL=postgresql://postgres:postgres@localhost:5432/postgres` and `JWT_SECRET` to a custom secret.

## Sample `.env`

```
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/postgres
JWT_SECRET=your_jwt_secret
```

# Building

To build the binary, clone the repository and `cd` into it. Then issue 

```
cargo build --release
```

# Running

To run the binary, make sure port 3000 is available. Then simply issue

```
docker-compose up -d
```

to start the postgres docker container, and

```
cargo run --release
```

to run the backend on port 3000.

### Run the project

uv run project1/main.py


### install 
uv tool install maturin

### use maturin
maturin new -b pyo3 rust_core
### build project using maturin
maturin develop --release -m rust_scrap_walmart_core/pyproject.toml

setup:
	uv venv
	uv pip install -r requirements.txt
	(cd rust_scrapwal && maturin develop)

.PHONY: setup

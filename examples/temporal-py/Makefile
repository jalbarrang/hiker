# Run `make test` (correct impl, passes) or `make test-buggy` (fails).
# Generated tests live in the gitignored .rascador-cache/python/ dir. pytest is
# given the explicit file path so it collects from the dotted cache dir (which
# its default recursion would otherwise skip).

GEN := cargo run -q -p rascador -- gen ../../.rascador/temporal.tent \
	--target python --module temporal -o .rascador-cache/python/test_generated.py
PYTEST := .venv/bin/pytest .rascador-cache/python/test_generated.py -q

.PHONY: gen test test-buggy

gen:
	$(GEN)

test: gen
	$(PYTEST)

test-buggy: gen
	RASCADOR_BUGGY=1 $(PYTEST)

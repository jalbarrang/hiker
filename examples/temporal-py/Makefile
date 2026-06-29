# Run `make test` (correct impl, passes) or `make test-buggy` (fails).
# Generated tests live in the gitignored .hiker-cache/python/ dir. pytest is
# given the explicit file path so it collects from the dotted cache dir (which
# its default recursion would otherwise skip).

GEN := cargo run -q -p hiker -- gen ../../.hiker/temporal.tent \
	--target python --module temporal -o .hiker-cache/python/test_generated.py
PYTEST := .venv/bin/pytest .hiker-cache/python/test_generated.py -q

.PHONY: gen test test-buggy

gen:
	$(GEN)

test: gen
	$(PYTEST)

test-buggy: gen
	HIKER_BUGGY=1 $(PYTEST)

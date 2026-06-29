# Make `import temporal` resolve when pytest runs the generated test that lives
# in the gitignored .hiker-cache/python/ directory. pytest auto-loads this
# conftest from the example root; we add the example dir to the import path.
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))

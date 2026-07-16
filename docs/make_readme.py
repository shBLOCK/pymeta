from pathlib import Path

import utils

utils.resolve_includes(
    Path("README.template.md"),
    Path("../README.md"),
    {
        "BACKEND_TEST": "/pymeta-proc-macro-backend/tests/backend"
    }
)

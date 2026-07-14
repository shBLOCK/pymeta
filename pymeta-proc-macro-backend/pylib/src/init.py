import importlib
from importlib.abc import MetaPathFinder, ExecutionLoader
import importlib.util


class PyMetaBuiltinsImporter(MetaPathFinder, ExecutionLoader):
    def __init__(self, files: dict):
        self.files = files
    
    def get_dir_or_file(self, module_name: str) -> dict | str | None:
        dof = self.files
        for segment in module_name.split("."):
            if not isinstance(dof, dict):
                return None
            dof = dof.get(segment)
        return dof

    def find_spec(self, fullname, path, target=None):
        if self.get_source(fullname) is not None:
            return importlib.util.spec_from_loader(fullname, self)
        return None

    def _get_source(self, fullname) -> tuple[str | None, str | None]:
        path = "<pylib>/" + '/'.join(fullname.split('.'))
        match self.get_dir_or_file(fullname):
            case str(src):
                return f"{path}.py", src
            case {"__init__": str(src)}:
                return f"{path}/__init__.py", src
        return None, None

    def get_filename(self, fullname):
        return self._get_source(fullname)[0]

    def get_source(self, fullname):
        return self._get_source(fullname)[1]
    
    def is_package(self, fullname):
        match self.get_dir_or_file(fullname):
            case {"__init__": str(_)}:
                return True
        return False

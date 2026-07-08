import importlib
from importlib.abc import InspectLoader, MetaPathFinder
import importlib.util


class PyMetaBuiltinsImporter(MetaPathFinder, InspectLoader):
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

    def get_source(self, fullname):
        match self.get_dir_or_file(fullname):
            case str(src):
                return src
            case {"__init__": str(src)}:
                return src
        return None

    def is_package(self, fullname):
        match self.get_dir_or_file(fullname):
            case {"__init__": str(_)}:
                return True
        return False

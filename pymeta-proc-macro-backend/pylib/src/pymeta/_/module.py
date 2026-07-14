import importlib
import sys
from importlib.abc import InspectLoader, MetaPathFinder
import importlib.util


class PyMetaModuleImporter(MetaPathFinder, InspectLoader):
    def __init__(self, path: str, modules: dict[str, str]):
        self.path = path
        self.modules = modules
        sys.meta_path.append(self)

    def kill(self):
        sys.meta_path.remove(self)
        for module in self.modules:
            name = f"{self.path}.{module}"
            if name in sys.modules:
                del sys.modules[name]

    @classmethod
    def kill_all(cls):
        for importer in [it for it in sys.meta_path if isinstance(it, cls)]:
            importer.kill()

    def find_spec(self, fullname, path, target=None):
        if self.get_source(fullname) is not None or self.is_package(fullname):
            return importlib.util.spec_from_loader(fullname, self)
        return None

    def get_source(self, fullname):
        if self.is_package(fullname):
            return ""
        if fullname.startswith(self.path):
            module = fullname[len(self.path) + 1:]
            return self.modules.get(module)
        return None

    def is_package(self, fullname):
        return self.path.startswith(fullname)

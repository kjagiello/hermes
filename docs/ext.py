from pathlib import Path
from urllib.parse import quote_plus


def define_env(env):
    docs_dir = env.variables.config["docs_dir"]
    env.variables["hermes_version"] = (
        "master"
        if (version := env.variables["extra"]["hermes_version"]) == "dev"
        else version
    )

    @env.macro
    def raw_github_url(path: str) -> str:
        tpl = env.variables["github_raw_url_tpl"]
        return tpl.format(version=env.variables["hermes_version"], path=path)

    @env.macro
    def import_file(path: str) -> str:
        with open(Path(docs_dir) / Path(path), "r") as f:
            return f.read()

    @env.filter
    def urlencode(val: str) -> str:
        "Reverse a string (and uppercase)"
        return quote_plus(val)

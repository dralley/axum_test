import aiohttp
import asyncio
import json
import glob
import requests
from pathlib import Path
from collections import defaultdict
from urllib.parse import urljoin


repos = defaultdict(dict)
# {
#   "cs8-appstream-aarch64": {
#       "base_url": "....",
#       "snapshots": ["20210414", ...],
#       "latest": "20210414",
#   }
# }
rpmrepo_id_to_our_id = {}

rpmrepo_base_url = "https://rpmrepo.osbuild.org/v2/mirror/public"

for repo_file in glob.glob("repo_definitions/*.json"):
    with open(repo_file, "r") as f:
        repo = json.loads(f.read())
        if repo["storage"] != "public":
            continue
        (platform, arch, repo_name) = repo["snapshot-id"].split("-", maxsplit=2)
        repo_id = f"{platform}-{repo_name}-{arch}"
        rpmrepo_id_to_our_id[repo["snapshot-id"]] = repo_id

        repos[repo_id]["platform"] = platform
        repos[repo_id]["arch"] = arch
        repos[repo_id]["repo"] = repo_name

        repos[repo_id]["snapshot_id"] = repo["snapshot-id"]
        repos[repo_id]["base_url"] = repo["base-url"]
        repos[repo_id]["snapshots"] = {}

with open("repo-list.json", "r") as f:
    snapshots = json.loads(f.read())
    available_repos = set(repo["snapshot_id"] for repo in repos.values())
    for snapshot in snapshots:
        (snapshot_id, snapshot_timestamp) = snapshot.rsplit("-", maxsplit=1)
        if snapshot_id not in available_repos:
            continue

        repo_id = rpmrepo_id_to_our_id[snapshot_id]
        repo = repos[repo_id]

        platform = repo["platform"]
        arch = repo["arch"]
        repo_name = repo["repo"]

        rpmrepo_snapshot_url = f"{rpmrepo_base_url}/{platform}/{platform}-{arch}-{repo_name}-{snapshot_timestamp}/"

        repo["snapshots"][snapshot_timestamp] = rpmrepo_snapshot_url

for (repo_name, data) in repos.items():
    max_timestamp = max(data["snapshots"].keys())
    data["snapshots"]["latest"] = data["snapshots"][max_timestamp]
    data["latest_snapshot"] = max_timestamp
    for snapshot_timestamp, snapshot_url in data["snapshots"].items():
        snapshot_path = Path("snapshots") / repo_name / snapshot_timestamp
        snapshot_path.mkdir(parents=True, exist_ok=True)

        repomd_path = snapshot_path / "repomd.xml"
        if repomd_path.exists():
            continue
        repomd_url = urljoin(snapshot_url, "repodata/repomd.xml")
        response = requests.get(repomd_url)
        if response.status_code != 200:
            snapshot_path.rmdir()
        else:
            repomd = response.text
            with open(repomd_path, "wt") as f:
                f.write(repomd)

with open("repo_data_dump.json", "wt") as f:
    f.write(json.dumps(repos, indent=4))

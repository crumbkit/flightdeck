"""Writes the documents that the fake board beside this file prints: a
board/query-report/v2 of the graph records, one of the change records, and
a board/log-report/v2 for each path the tests log. Run it from
tests/fixtures/board after a change to the example graphs in graphs/.
Every change, commit and title in them is invented."""

import copy
import json
import os

NAMES = ["development-flow", "small-flow", "weekly-suite-loop"]
COLUMNS = {"columns": ["logical", "type", "status", "code", "body"]}


def query_report(rows):
    return {"schema": "board/query-report/v2", "data": rows, "truncated": False, "summary": COLUMNS}


def log_report(commits):
    data = [{"commit": c, "time": t, "subject": "board put --batch (1 records)", "blob": b} for c, t, b in commits]
    summary = {"at": "c0ffee" + "0" * 34, "walked": 40, "complete": True, "location": None}
    return {"schema": "board/log-report/v2", "data": data, "truncated": False, "summary": summary}


def write(path, document):
    with open(path, "w") as out:
        out.write(json.dumps(document) + "\n")


graphs = {name: json.load(open(f"graphs/{name}.json")) for name in NAMES}
tangled = copy.deepcopy(graphs["small-flow"])
tangled["title"] = "Tangled flow"
tangled["edges"][2]["to"] = "nowhere"
GRAPH = "flightdeck/workflow-graph/v1"
rows = [
    ["/workflow-graphs/broken", GRAPH, "degraded", "board::schema::invalid", json.dumps({"$type": GRAPH, "title": "Broken"}) + "\n"],
    ["/workflow-graphs/development-flow", GRAPH, "ok", None, json.dumps(graphs["development-flow"], indent=2) + "\n"],
    ["/workflow-graphs/small-flow", GRAPH, "ok", None, json.dumps(graphs["small-flow"], indent=2) + "\n"],
    ["/workflow-graphs/tangled", GRAPH, "ok", None, json.dumps(tangled, indent=2) + "\n"],
    ["/workflow-graphs/weekly-suite-loop", GRAPH, "ok", None, json.dumps(graphs["weekly-suite-loop"], indent=2) + "\n"],
]
write("fake/graphs.json", query_report(rows))

CURRENT = "/flows/development/3029fae590f5a628"
OLDER = "/flows/development/23eb709b901bc3e3"
SMALL = "/flows/small/f5df48f0a0456d93"
CHANGES = [
    ("/changes/sample-plan-current", "The page shows a sample", "plan", CURRENT),
    ("/changes/sample-plan-older", "A plan on an older flow", "plan", OLDER),
    ("/changes/sample-plan-older-too", "Another plan on an older flow", "plan", OLDER),
    ("/changes/sample-build-current", "A build on the current flow", "build", CURRENT),
    ("/changes/sample-review-older", "A review on an older flow", "review", OLDER),
    ("/changes/sample-paused", "A paused change", "paused", CURRENT),
    ("/changes/sample-done", "A merged change", "done", CURRENT),
    ("/changes/sample-small-build", "A small change", "build", SMALL),
]
CHANGE = "changes/change/v2"
change_rows = []
for path, title, phase, flow in CHANGES:
    record = {"$type": CHANGE, "flow": flow, "mount": "/repos/sample", "phase": phase, "title": title}
    change_rows.append([path, CHANGE, "ok", None, json.dumps(record, indent=2) + "\n"])
degraded = {"$type": CHANGE, "flow": CURRENT, "phase": "plan", "title": "A degraded change"}
change_rows.append(["/changes/sample-degraded", CHANGE, "degraded", "board::ref::dangling", json.dumps(degraded) + "\n"])
odd = {"$type": CHANGE, "flow": CURRENT, "phase": 3, "title": "A change whose phase is a number"}
change_rows.append(["/changes/sample-odd-phase", CHANGE, "ok", None, json.dumps(odd) + "\n"])
change_rows.sort()
write("fake/changes.json", query_report(change_rows))

os.makedirs("fake/log", exist_ok=True)
LOGS = {
    "workflow-graphs_development-flow": [
        ("aa" + "1" * 38, "2026-11-24T10:00:00Z", "1" * 40),
        ("aa" + "2" * 38, "2026-11-23T10:00:00Z", "2" * 40),
        ("aa" + "3" * 38, "2026-11-22T04:45:00Z", "3" * 40),
    ],
    "workflow-graphs_small-flow": [("bb" + "1" * 38, "2026-11-22T04:45:00Z", "4" * 40)],
    "workflow-graphs_weekly-suite-loop": [("cc" + "1" * 38, "2026-11-22T04:45:00Z", "5" * 40)],
    "flows_development_3029fae590f5a628": [("9a84281ac0df6d68d5223deba53da18d4a780945", "2026-11-19T18:30:14Z", "6" * 40)],
    "flows_small_f5df48f0a0456d93": [("07dee52642e50bb3f69169d5cafff3e30de876b2", "2026-11-19T18:30:14Z", "7" * 40)],
}
# The weekly loop's plan has not changed since the reading, so the walk
# from the head and the walk from the reading's commit find one newest
# commit.
PLAN = "changes_sample-weekly-suite-loop_plans_20261122T012641103210Z.md"
READ_AT = "4e0e85285b4819d32b04cd569fe57a925558359d"
LOGS[PLAN] = [(READ_AT, "2026-11-22T01:26:41Z", "8" * 40)]
LOGS[f"{PLAN}@{READ_AT}"] = LOGS[PLAN]
# A reading at an earlier commit, from which the walk finds an older
# change, so the plan changed after that reading.
EARLIER = "3d0e85285b4819d32b04cd569fe57a925558359d"
LOGS[f"{PLAN}@{EARLIER}"] = [("2c" + "0" * 38, "2026-11-21T09:00:00Z", "9" * 40)]
for key, commits in LOGS.items():
    write(f"fake/log/{key}.json", log_report(commits))

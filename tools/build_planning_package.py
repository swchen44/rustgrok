from pathlib import Path
import json, re, hashlib, zipfile, unicodedata

import argparse

parser = argparse.ArgumentParser(description="Validate and build the v0.6 rustgrok planning package; no product tests are executed.")
parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
parser.add_argument("--output-dir", type=Path)
args = parser.parse_args()
root = args.root.resolve()
base = args.output_dir.resolve() if args.output_dir else root / "dist"
if base == root or (root in base.parents and base != root / "dist"):
    raise ValueError("Output inside the repository must use its dist directory.")
base.mkdir(parents=True, exist_ok=True)

def package_files():
    excluded = {".git", "dist", "target", "node_modules", "__pycache__"}
    return sorted(p for p in root.rglob("*") if p.is_file() and not excluded.intersection(p.relative_to(root).parts))
errors = []
files = [p for p in package_files() if p.name not in ("planning-manifest.json", "planning-quality-report.json")]
for p in files:
    s = p.read_text()
    if any(unicodedata.category(c) == "Cf" for c in s):
        errors.append("Invisible formatting character: " + str(p))
    if p.suffix == ".json":
        json.loads(s)
    if p.suffix == ".md":
        for label, dest in re.findall(r"\[([^\]\n]+)\]\(([^)\n]+)\)", s):
            if not dest.startswith(("http://", "https://", "#", "mailto:")):
                if not (p.parent / dest.split("#")[0]).exists():
                    errors.append("Broken link: " + str(p) + " -> " + dest)
matrix = json.loads((root / "tests/matrix.json").read_text())
tests = matrix["tests"]
reqs = {f"FR-{i:03}" for i in range(1, 23)} | {f"NFR-{i:03}" for i in range(1, 17)}
assert {r for t in tests for r in t["req_ids"]} == reqs
assert len({t["test_id"] for t in tests}) == 80
assert all(t["status"] == "planned" and t["release_gate"] == "G6" for t in tests)
for t in tests:
    for key in ("test_id", "req_ids", "preconditions", "steps", "expected", "evidence", "gate", "acceptance_gate", "release_gate", "status"):
        assert t.get(key), (t["test_id"], key)
wps = set(re.findall(r"^\| (WP-[A-E]\d\d) \|", (root / "docs/04-implementation-plan.md").read_text(), re.M))
assert len(wps) == 44
for row in json.loads((root / "tests/traceability.json").read_text())["requirements"]:
    assert set(row["work_packages"]) <= wps
trace = json.loads((root / "tests/traceability.json").read_text())["requirements"]
assert len(trace) == len(reqs) and {r["req_id"] for r in trace} == reqs
all_tests = {t["test_id"] for t in tests}
mapped_tests, mapped_wps = set(), set()
for row in trace:
    expected_tests = {t["test_id"] for t in tests if row["req_id"] in t["req_ids"]}
    assert set(row["test_ids"]) == expected_tests, row["req_id"]
    assert row["work_packages"] and row["status"] == "planned" and row["evidence_status"] == "not_run"
    for doc in row["design_docs"]:
        assert (root / doc).is_file(), doc
    mapped_tests |= set(row["test_ids"])
    mapped_wps |= set(row["work_packages"])
assert mapped_tests == all_tests and mapped_wps == wps
srs_text = (root / "docs/02-requirements.md").read_text()
srs_ids = set(re.findall(r"^\| ((?:N?FR)-\d{3}) \|", srs_text, re.M))
assert srs_ids == reqs, srs_ids ^ reqs
wp_text = (root / "docs/04-implementation-plan.md").read_text()
dependencies = {}
for line in wp_text.splitlines():
    if re.match(r"^\| WP-[A-E]\d\d \|", line):
        cells = line.strip().strip("|").split("|")
        dependencies[cells[0].strip()] = set(re.findall(r"WP-[A-E]\d\d", cells[-1]))
assert set(dependencies) == wps
for wp, deps in dependencies.items():
    assert deps <= wps and wp not in deps, (wp, deps)
visited, active = set(), set()
def visit(wp):
    assert wp not in active, ("dependency cycle", wp)
    if wp in visited: return
    active.add(wp)
    for dep in dependencies[wp]: visit(dep)
    active.remove(wp)
    visited.add(wp)
for wp in dependencies: visit(wp)
decisions = (root / "docs/08-decision-register.md").read_text()
decision_rows = re.findall(r"^\| (D\d\d) \|", decisions, re.M)
assert set(decision_rows) == {f"D{i:02}" for i in range(1, 22)}
assert len(decision_rows) == 21
for did in ("D01", "D02", "D11", "D19", "D21"):
    assert re.search(r"^\| " + did + r" \| USER_CONFIRMED", decisions, re.M)
known_ids = reqs | all_tests
for path in files:
    if path.suffix != ".md": continue
    for token in re.findall(r"\b[A-Z]{2,4}-\d{3}\b", path.read_text()):
        if token.startswith("ADR-") or token == "SHA-256": continue
        assert token in known_ids, (str(path), token)

assert not errors, errors
report = {
    "checked_at": "2026-10-25", "scope": "planning artifact structure only; not a product test run",
    "files_checked": len(files), "requirements_with_test_specs": 38, "planned_test_specs": 80,
    "work_packages": 44, "product_tests_executed": False, "errors": [],
    "checks": ["UTF-8 and no invisible formatting controls", "JSON parses", "Markdown local links resolve",
               "Unique test IDs", "38 requirements mapped", "Required test-spec fields present",
               "All tests planned; release gate G6", "Traceability WP IDs resolve", "38 SRS IDs match requirements",
               "All 80 tests and 44 work packages map bidirectionally",
               "Work-package dependency graph is acyclic", "D01/D02/D11/D19/D21 confirmed; D01-D21 unique",
               "Referenced requirement/test IDs exist"]
}
(root / "planning-quality-report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")

order = [root / "README.md"] + sorted(root.glob("docs/[0-9][0-9]-*.md")) + [
    root / "docs/sources.md", root / "docs/adr/0001-runtime-and-core-reuse.proposed.md",
    root / "docs/adr/0002-rust-core-and-compatibility.md",
    root / "tests/README.md", root / "tests/fixtures/README.md", root / "tests/evidence/README.md", root / "AGENTS.md",
    root / "crates/README.md", root / "npm/README.md", root / "assets/README.md", root / ".github/workflows/README.md",
    root / "tests/performance/README.md", root / "tools/README.md"]
def anchor(p):
    return "doc-" + re.sub("[^a-z0-9]+", "-", p.relative_to(root).as_posix().lower()).strip("-")
anchors = {p.resolve(): anchor(p) for p in order}
appendices = [("upstream/research-lock.json", "research-lock", "研究版本鎖定資料"),
              ("upstream/source-review-manifest.json", "source-review-manifest", "原始碼查核 manifest"),
              ("upstream/rust-research-lock.json", "rust-research-lock", "Rust 候選與相容性研究鎖定"),
              ("tests/fixtures/corpus-manifest.template.json", "corpus-template", "待建立語料的規格模板"),
              ("tests/traceability.json", "traceability-json", "機器可讀需求對照"),
              ("tests/performance/search-benchmark-manifest.template.json", "search-benchmark-template", "搜尋效能量測模板"),
              ("distribution/release-manifest.template.json", "release-manifest-template", "GitHub Release產物追溯模板")]
anchors[(root / "tests/matrix.json").resolve()] = "test-matrix"
for rel, a, title in appendices:
    anchors[(root / rel).resolve()] = a
def convert_links(s, p):
    def sub(m):
        dest = m.group(2)
        if dest.startswith(("http://", "https://", "#", "mailto:")):
            return m.group(0)
        target = (p.parent / dest.split("#")[0]).resolve()
        return "[" + m.group(1) + "](#" + anchors[target] + ")" if target in anchors else m.group(0)
    return re.sub(r"\[([^\]\n]+)\]\(([^)\n]+)\)", sub, s)

parts = ["# OpenGrok Rust Host 研究與開發計畫 v0.6\n\n2026-10-25（Asia/Taipei）\n\n"
         "**Rust、bundled helpers、搜尋優先、local＋LAN約5人且無權限管理已確認；macOS先開發及完整驗證，其後Ubuntu24.04 amd64適配與完整原生重驗。Public swchen44/rustgrok已建立，首輪Mac已定M1／arm64。產品與正式產品Release尚未建立。**\n\n"
         "本檔是可單獨閱讀的完整計畫合訂本。ZIP 保留 docs/tests/範本等 Monorepo 規劃目錄。各章 Sxx 是該章來源編號。附錄包含 80 個測試的具體步驟，不是測試通過報告。\n\n## 目錄\n"]
for p in order:
    title = p.read_text().splitlines()[0].lstrip("# ").strip()
    parts.append("- [" + title + "](#" + anchor(p) + ")")
parts.append("- [完整 80 項測試規格](#test-matrix)")
for rel, a, title in appendices:
    parts.append("- [" + title + "](#" + a + ")")
for p in order:
    parts.append('\n\n---\n\n<a id="' + anchor(p) + '"></a>\n\n' + convert_links(p.read_text(), p))
parts.append('\n\n---\n\n<a id="test-matrix"></a>\n\n# 附錄：80 項完整測試規格\n\n'
             "所有項目均為 planned／尚未執行。Gate G0–G6 與正文一致。\n\n"
             + "\n\n".join(matrix["notes"]) + "\n")
for t in tests:
    parts.append("## " + t["test_id"] + "：" + t["title"] + "\n\n需求：" + ", ".join(t["req_ids"])
                 + "。最早 Gate：" + t["gate"] + "；驗收 Gate：" + t["acceptance_gate"] + "；交接 Gate：" + t["release_gate"]
                 + "。\n\n平台：" + t["platform_scope"]
                 + "\n\n### 前置條件\n\n" + "\n".join("- " + x for x in t["preconditions"])
                 + "\n\n### 操作\n\n" + "\n".join(str(i + 1) + ". " + x for i, x in enumerate(t["steps"]))
                 + "\n\n### 預期結果\n\n" + t["expected"]
                 + "\n\n### 未來證據\n\n" + "\n".join("- " + x for x in t["evidence"])
                 + "\n\n狀態：planned。" + t["execution_note"] + "\n")
for rel, a, title in appendices:
    parts.append('\n\n---\n\n<a id="' + a + '"></a>\n\n# 附錄：' + title
                 + "\n\n~~~json\n" + (root / rel).read_text() + "~~~\n")
book = base / "OpenGrok-Rust-Host-Planning-v0.6.md"
book.write_text("\n".join(parts))
for target in re.findall(r"\]\(#([^)]*)\)", book.read_text()):
    assert '<a id="' + target + '"></a>' in book.read_text(), target
manifest = []
for p in package_files():
    if p.is_file() and p.name != "planning-manifest.json":
        manifest.append({"path": p.relative_to(root).as_posix(), "bytes": p.stat().st_size, "sha256": hashlib.sha256(p.read_bytes()).hexdigest()})
(root / "planning-manifest.json").write_text(json.dumps({
    "version": "0.6", "kind": "planning_package_not_runtime_distribution", "generated_date": "2026-10-25",
    "files": manifest, "manifest_excludes_itself": True}, ensure_ascii=False, indent=2) + "\n")
archive = base / "OpenGrok-Rust-Host-Monorepo-Plan-v0.6.zip"
with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as z:
    for p in package_files():
        if p.is_file():
            info = zipfile.ZipInfo("opengrok-host-planning/" + p.relative_to(root).as_posix(), (2026, 10, 25, 0, 0, 0))
            info.external_attr = 0o100644 << 16
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, p.read_bytes())
with zipfile.ZipFile(archive) as z:
    assert z.testzip() is None
    for f in manifest:
        assert hashlib.sha256(z.read("opengrok-host-planning/" + f["path"])).hexdigest() == f["sha256"]
print(json.dumps({"book": str(book), "book_bytes": book.stat().st_size, "zip": str(archive),
                  "zip_bytes": archive.stat().st_size, "zip_files": len(manifest) + 1,
                  "requirements": 38, "test_specs": 80, "work_packages": 44,
                  "quality_and_archive_integrity": "passed", "product_tests_executed": False}, ensure_ascii=False, indent=2))

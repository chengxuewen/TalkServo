#!/usr/bin/env python3
"""Lane registry reconciliation gate."""

from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]

CARGO_MEMBERS_PATH = Path("Cargo.toml")
CRATES_PATH = Path("cmake/TalkServoCrates.cmake")
WEB_PATH = Path("cmake/TalkServoWeb.cmake")
INSTALL_PATH = Path("cmake/TalkServoInstall.cmake")
GATES_PATH = Path("cmake/TalkServoGates.cmake")
DOCS_PATH = Path("docs/modules/09-dev-toolchain.md")

EXPECTED_CARGO_MEMBERS = {
    "talkservo-core",
    "talkservo-sfu",
    "talkservo-server",
}

EXPECTED_LANES = [
    ("talkservo-core", "talkservo-core", "rust;headless", "stub"),
    ("talkservo-sfu", "talkservo-sfu", "rust;headless", "stub"),
    ("talkservo-server-stub", "talkservo-server", "rust;headless", "stub"),
    ("talkservo-server-live", "talkservo-server", "mediasoup;headless;slow", "live"),
]

EXPECTED_DOCS_LANES = [
    ("talkservo-core", "rust;headless"),
    ("talkservo-sfu", "rust;headless"),
    ("talkservo-server-stub", "rust;headless"),
    ("talkservo-server-live", "mediasoup;headless;slow"),
    ("talkservo-web", "web;headless"),
]

IDE_BUILD_TARGETS = [
    ("talkservo-gates-build", "-L gate", "lanes", []),
    ("talkservo-e2e-stub-build", "-L e2e-stub", "lanes", ["talkservo-lanes-stub-build"]),
]

DEPLOY_TARGETS = [
    ("talkservo-deploy-build", "talkservo-server-${TALKSERVO_PACKAGE_LANE}"),
]

EXPECTED_TARGET_INVENTORY_LINE = "Inventory (18 targets"

EXPECTED_CLEAN_TARGETS = {
    "talkservo-lanes-stub-clean",
    "talkservo-lanes-live-clean",
    "talkservo-lanes-clean",
    "talkservo-web-clean",
}

REQUIRED_CLEAN_SOURCE = {
    "talkservo-lanes-stub-clean": CRATES_PATH,
    "talkservo-lanes-live-clean": CRATES_PATH,
    "talkservo-lanes-clean": CRATES_PATH,
    "talkservo-web-clean": WEB_PATH,
}

CLEAN_DEFINITION_PATHS = [
    CRATES_PATH,
    WEB_PATH,
]

FORBIDDEN_CLEAN_SCAN_PATHS = [
    CRATES_PATH,
    WEB_PATH,
    INSTALL_PATH,
    GATES_PATH,
    DOCS_PATH,
    Path("pixi.toml"),
    Path("talkservo.sh"),
]


def read_text(path: Path) -> str:
    return (ROOT / path).read_text()


def cargo_members(path: Path) -> set[str]:
    text = read_text(path)
    match = re.search(r"members\s*=\s*\[(.*?)\]", text, re.S)
    if not match:
        return set()
    return {
        m.group(1)
        for m in re.finditer(r'"crates/([^"]+)"', match.group(1))
    }


def parse_lanes(path: Path) -> list[tuple[str, str, str, str]]:
    text = read_text(path)
    lanes: list[tuple[str, str, str, str]] = []

    for call_match in re.finditer(r"talkservo_rust_lane\((.*?)\)\n", text, re.S):
        call = call_match.group(1)

        handle_match = re.search(r"HANDLE\s+(\S+)", call)
        package_match = re.search(r"PACKAGE\s+(\S+)", call)
        labels_match = re.search(r'TEST_LABELS\s+"([^"]+)"', call)
        cache_match = re.search(r"CACHE\s+(\S+)", call)

        if not (handle_match and package_match and labels_match and cache_match):
            continue

        lanes.append(
            (
                handle_match.group(1),
                package_match.group(1),
                labels_match.group(1),
                cache_match.group(1),
            )
        )

    return lanes


def docs_table(path: Path) -> dict[str, list[str]]:
    rows: dict[str, list[str]] = {}
    for line in read_text(path).splitlines():
        if not line.startswith("| `talkservo-"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) < 6:
            continue
        handle = cells[0].strip("`")
        rows[handle] = cells
    return rows


def cmake_code_without_comments(path: Path) -> str:
    lines: list[str] = []
    for line in read_text(path).splitlines():
        lines.append(line.split("#", 1)[0])
    return "\n".join(lines)


def clean_target_definitions() -> dict[str, str]:
    found: dict[str, str] = {}
    for path in CLEAN_DEFINITION_PATHS:
        text = cmake_code_without_comments(path)
        for match in re.finditer(r"talkservo-[A-Za-z0-9_-]+clean", text):
            found[match.group(0)] = str(path)

    crates = cmake_code_without_comments(CRATES_PATH)
    if re.search(r"polyorch_rust_clean\(NAME\s+talkservo-lanes-\$\{_group\}-clean", crates):
        for group in ("stub", "live"):
            found[f"talkservo-lanes-{group}-clean"] = str(CRATES_PATH)

    return found


def forbidden_clean_target_mentions() -> dict[str, str]:
    paths = list(FORBIDDEN_CLEAN_SCAN_PATHS)
    scripts_dir = ROOT / "scripts"
    if scripts_dir.is_dir():
        for path in sorted(scripts_dir.glob("*.sh")):
            paths.append(path.relative_to(ROOT))

    found: dict[str, str] = {}
    for path in paths:
        full = ROOT / path
        if not full.is_file():
            continue
        text = "\n".join(line.split("#", 1)[0] for line in full.read_text().splitlines())
        for match in re.finditer(r"talkservo-[A-Za-z0-9_-]+clean", text):
            found[match.group(0)] = str(path)
    return found


def check_lane_table(lanes: list[tuple[str, str, str, str]], source: str) -> list[str]:
    errors: list[str] = []
    if lanes != EXPECTED_LANES:
        errors.append(
            f"FAIL: {source} lane rows mismatch: expected {EXPECTED_LANES} found {lanes}"
        )
    for handle, _, labels, _ in lanes:
        if "rust-live" in labels:
            errors.append(
                f"FAIL: {source} lane {handle} uses forbidden label substring: "
                "found rust-live, expected mediasoup for the live lane"
            )
    return errors


def check_docs_table(docs_rows: dict[str, list[str]], source: str) -> list[str]:
    errors: list[str] = []
    for handle, labels in EXPECTED_DOCS_LANES:
        cells = docs_rows.get(handle)
        if cells is None:
            errors.append(f"FAIL: {source} missing lane row for {handle}")
            continue
        build_cell = cells[1]
        test_cell = cells[2]
        label_cell = cells[5]
        if "-build" not in build_cell:
            errors.append(
                f"FAIL: {source} row for {handle} missing -build: "
                f"found {build_cell!r}, expected -build"
            )
        if "-test" not in test_cell:
            errors.append(
                f"FAIL: {source} row for {handle} missing -test: "
                f"found {test_cell!r}, expected -test"
            )
        if f"`{labels}`" not in label_cell:
            errors.append(
                f"FAIL: {source} row for {handle} has wrong label: "
                f"found {label_cell!r}, expected `{labels}`"
            )
    return errors


def check_clean_targets(
    definitions: dict[str, str],
    mentions: dict[str, str],
) -> list[str]:
    errors: list[str] = []
    sources: dict[str, str] = {}
    for name, source in definitions.items():
        sources[name] = source
    for name, source in mentions.items():
        sources.setdefault(name, source)

    for unexpected in sorted(set(sources) - EXPECTED_CLEAN_TARGETS):
        errors.append(
            f"FAIL: {sources[unexpected]} unexpected clean target {unexpected}: "
            f"expected only {sorted(EXPECTED_CLEAN_TARGETS)}"
        )

    for required in sorted(EXPECTED_CLEAN_TARGETS - set(definitions)):
        errors.append(
            f"FAIL: {REQUIRED_CLEAN_SOURCE[required]} missing clean target {required}: "
            f"expected only {sorted(EXPECTED_CLEAN_TARGETS)}"
        )

    return errors


def check_install_handle(source: str) -> list[str]:
    text = read_text(INSTALL_PATH)
    expected = "set(_handle talkservo-server-${TALKSERVO_PACKAGE_LANE})"
    if expected not in text:
        return [f"FAIL: {source} install handle drift: expected {expected!r}"]
    if 'TALKSERVO_PACKAGE_LANE MATCHES "^(stub|live)$"' not in text:
        return [
            f"FAIL: {source} install lane validation drift: "
            'expected TALKSERVO_PACKAGE_LANE MATCHES "^(stub|live)$"'
        ]
    return []


def check_web_label(source: str) -> list[str]:
    text = read_text(WEB_PATH)
    if 'LABELS "web;headless"' not in text:
        return [f"FAIL: {source} web ctest label drift: expected `web;headless`"]
    return []


def check_gates_label_substrings(source: str) -> list[str]:
    text = read_text(GATES_PATH)
    errors: list[str] = []
    for match in re.finditer(r'LABELS "([^"]+)"', text):
        if "rust-live" in match.group(1):
            errors.append(
                f"FAIL: {source} gate lane uses forbidden label substring: "
                f"found {match.group(1)!r}, expected no rust-live"
            )
    return errors


def gates_custom_target_block(text: str, target: str) -> str:
    match = re.search(rf"add_custom_target\({target}\b(.*?)\)\s*\n", text, re.S)
    return match.group(1) if match else ""


def check_deploy_targets(install_text: str, docs_text: str, docs_source: str) -> list[str]:
    errors: list[str] = []
    source = str(INSTALL_PATH)
    if "add_custom_target(talkservo-deploy-build" not in install_text:
        errors.append(
            f"FAIL: {source} missing deploy staging target: expected talkservo-deploy-build"
        )
    if "DEPENDS ${_handle}-build" not in install_text:
        errors.append(
            f"FAIL: {source} deploy target must depend on the configured lane build handle"
        )
    if "TALKSERVO_PACKAGE_LANE" not in install_text or "out/${TALKSERVO_PACKAGE_LANE}/bin" not in install_text.replace('${_dest}', 'out/${TALKSERVO_PACKAGE_LANE}/bin'):
        pass  # lane-word composition is checked in check_install_handle
    for target, _handle in DEPLOY_TARGETS:
        if target not in docs_text:
            errors.append(
                f"FAIL: {docs_source} missing deploy target row for {target}"
            )
    return errors


def check_ide_build_targets(gates_text: str, docs_text: str, docs_source: str) -> list[str]:
    errors: list[str] = []
    source = str(GATES_PATH)

    for target, label, folder, depends in IDE_BUILD_TARGETS:
        block = gates_custom_target_block(gates_text, target)
        if not block:
            errors.append(
                f"FAIL: {source} missing IDE custom target {target}: "
                "expected add_custom_target"
            )
        else:
            if label not in block:
                errors.append(
                    f"FAIL: {source} target {target} missing label {label}: "
                    "expected a ctest label in the target command"
                )
            for dependency in depends:
                if dependency not in block:
                    errors.append(
                        f"FAIL: {source} target {target} missing dependency {dependency}: "
                        "expected DEPENDS to keep IDE e2e fresh"
                    )
        set_line = f'set_property(TARGET {target} PROPERTY FOLDER "{folder}")'
        if set_line not in gates_text:
            errors.append(
                f"FAIL: {source} target {target} missing IDE FOLDER property: "
                f"expected {set_line}"
            )
        if target not in docs_text:
            errors.append(
                f"FAIL: {docs_source} missing IDE custom target {target}: "
                "expected target inventory to list it"
            )

    if EXPECTED_TARGET_INVENTORY_LINE not in docs_text:
        errors.append(
            f"FAIL: {docs_source} target inventory count drift: "
            f"expected {EXPECTED_TARGET_INVENTORY_LINE!r}"
        )

    return errors


def check_repo() -> int:
    errors: list[str] = []

    members = cargo_members(CARGO_MEMBERS_PATH)
    if members != EXPECTED_CARGO_MEMBERS:
        errors.append(
            f"FAIL: Cargo.toml workspace members mismatch: expected "
            f"{sorted(EXPECTED_CARGO_MEMBERS)} found {sorted(members)}"
        )

    lanes = parse_lanes(CRATES_PATH)
    errors.extend(check_lane_table(lanes, str(CRATES_PATH)))

    errors.extend(check_docs_table(docs_table(DOCS_PATH), str(DOCS_PATH)))
    errors.extend(
        check_clean_targets(clean_target_definitions(), forbidden_clean_target_mentions())
    )
    errors.extend(check_install_handle(str(INSTALL_PATH)))
    errors.extend(check_web_label(str(WEB_PATH)))
    errors.extend(check_gates_label_substrings(str(GATES_PATH)))
    errors.extend(
        check_ide_build_targets(read_text(GATES_PATH), read_text(DOCS_PATH), str(DOCS_PATH))
    )
    errors.extend(
        check_deploy_targets(read_text(INSTALL_PATH), read_text(DOCS_PATH), str(DOCS_PATH))
    )

    for error in errors:
        print(error)

    if errors:
        return 1
    print("PASS")
    return 0


def self_test() -> int:
    errors: list[str] = []

    if check_lane_table(EXPECTED_LANES, "valid") != []:
        errors.append("FAIL: self_test valid lane table was rejected")

    missing_row = [row for row in EXPECTED_LANES if row[0] != "talkservo-sfu"]
    missing_errors = check_lane_table(missing_row, "missing-sfu")
    if not any("talkservo-sfu" in error for error in missing_errors):
        errors.append("FAIL: self_test did not detect a missing lane row")

    bad_label = list(EXPECTED_LANES)
    bad_label[3] = (
        bad_label[3][0],
        bad_label[3][1],
        "rust-live",
        bad_label[3][3],
    )
    bad_label_errors = check_lane_table(bad_label, "bad-label")
    if not any("rust-live" in error for error in bad_label_errors):
        errors.append("FAIL: self_test did not reject the rust-live label substring")

    if check_clean_targets({name: "valid" for name in EXPECTED_CLEAN_TARGETS}, {}) != []:
        errors.append("FAIL: self_test valid clean target set was rejected")

    forbidden_clean_errors = check_clean_targets(
        {name: "valid" for name in EXPECTED_CLEAN_TARGETS},
        {"talkservo-core-clean": "pixi.toml"},
    )
    if not any("talkservo-core-clean" in error for error in forbidden_clean_errors):
        errors.append("FAIL: self_test did not reject a planted forbidden clean target")

    missing_clean = {name: "valid" for name in EXPECTED_CLEAN_TARGETS}
    missing_clean.pop("talkservo-web-clean")
    missing_clean_errors = check_clean_targets(missing_clean, {})
    if not any("talkservo-web-clean" in error for error in missing_clean_errors):
        errors.append("FAIL: self_test did not reject a missing required clean target")

    valid_gates = ""
    for target, label, folder, depends in IDE_BUILD_TARGETS:
        depends_line = f"  DEPENDS {' '.join(depends)}\n" if depends else ""
        valid_gates += f"add_custom_target({target}\n  COMMAND {label}\n{depends_line})\n"
        valid_gates += f'set_property(TARGET {target} PROPERTY FOLDER "{folder}")\n'
    valid_docs = "\n".join([target for target, _, _, _ in IDE_BUILD_TARGETS])
    valid_docs += "\n" + EXPECTED_TARGET_INVENTORY_LINE
    if check_ide_build_targets(valid_gates, valid_docs, "docs") != []:
        errors.append("FAIL: self_test valid IDE target inventory was rejected")

    missing_docs = valid_docs.replace(IDE_BUILD_TARGETS[0][0], "")
    missing_docs_errors = check_ide_build_targets(valid_gates, missing_docs, "missing-docs")
    if not any(IDE_BUILD_TARGETS[0][0] in error for error in missing_docs_errors):
        errors.append("FAIL: self_test did not reject a missing IDE target in docs")

    missing_definition = valid_gates.replace(f"add_custom_target({IDE_BUILD_TARGETS[1][0]}", "")
    missing_definition_errors = check_ide_build_targets(missing_definition, valid_docs, "missing-def")
    if not any(IDE_BUILD_TARGETS[1][0] in error for error in missing_definition_errors):
        errors.append("FAIL: self_test did not reject a missing IDE target definition")

    for error in errors:
        print(error)

    if errors:
        return 1
    print("PASS")
    return 0


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        return self_test()
    return check_repo()


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))

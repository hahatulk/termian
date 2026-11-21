import pathlib
import re
import sys
import xml.etree.ElementTree as ET

destination = pathlib.Path(sys.argv[1])
destination.mkdir(parents=True, exist_ok=True)
namespace = "http://schemas.android.com/apk/res/android"
ET.register_namespace("android", namespace)

def make(parent, tag, **fields):
    attributes = {f"{{{namespace}}}{name}": str(value) for name, value in fields.items()}
    return ET.Element(tag, attributes) if parent is None else ET.SubElement(parent, tag, attributes)

def shapes(parent, node, style):
    tag = node.tag.split("}")[-1]
    current = dict(style)
    for key in ("fill", "stroke", "stroke-width", "stroke-linecap", "stroke-linejoin", "fill-rule"):
        if key in node.attrib:
            current[key] = node.attrib[key]
    if tag in ("svg", "g"):
        transform = node.get("transform")
        if transform:
            match = re.fullmatch(r"translate\(([-.\d]+)(?:[ ,]+([-.\d]+))?\)", transform)
            if not match:
                raise ValueError(transform)
            parent = make(parent, "group", translateX=match[1], translateY=match[2] or 0)
        for child in node:
            shapes(parent, child, current)
    elif tag in ("path", "rect"):
        path = node.get("d")
        if tag == "rect":
            x, y, w, h, r = map(float, (node.get(a, "0") for a in ("x", "y", "width", "height", "rx")))
            if r:
                path = f"M{x+r},{y} H{x+w-r} Q{x+w},{y} {x+w},{y+r} V{y+h-r} Q{x+w},{y+h} {x+w-r},{y+h} H{x+r} Q{x},{y+h} {x},{y+h-r} V{y+r} Q{x},{y} {x+r},{y} Z"
            else:
                path = f"M{x},{y} h{w} v{h} h{-w} Z"
        color = lambda value: "#00000000" if value == "none" else "#FFFFFFFF"
        attrs = {"pathData": path, "fillColor": color(current.get("fill", "black"))}
        if current.get("fill-rule") == "evenodd":
            attrs["fillType"] = "evenOdd"
        if current.get("stroke", "none") != "none":
            attrs.update(strokeColor=color(current["stroke"]), strokeWidth=current.get("stroke-width", "1"),
                         strokeLineCap=current.get("stroke-linecap", "butt"), strokeLineJoin=current.get("stroke-linejoin", "miter"))
        make(parent, "path", **attrs)

for name in ("vaults", "hosts", "sftp", "keychain", "identity", "logs", "port_forwarding", "snippets", "known_hosts", "terminal",
             "privacy_lock", "workspace", "plus", "search", "dots", "new_host", "linux", "debian", "ubuntu",
             "group", "file", "upload", "download", "cloud_sync", "local", "bookmark", "bookmark_filled",
             "grid", "list", "arrow_left", "arrow_right", "sftp_arrow", "sftp_eye_off", "link", "caret_down", "collapse_right", "new_tab",
             "person", "env", "cloud_download",
             "edit", "sorting", "backspace", "check", "connect", "side_panel", "broadcast"):
    root = ET.parse(f"assets/icons/{name}.svg").getroot()
    x, y, w, h = map(float, root.get("viewBox").split())
    dimension = max(w, h)
    icon = make(None, "vector", width="24dp", height="24dp", viewportWidth=dimension, viewportHeight=dimension)
    icon_group = make(icon, "group", translateX=(dimension-w)/2-x, translateY=(dimension-h)/2-y)
    shapes(icon_group, root, {})
    if not icon.findall(".//path"):
        raise ValueError(name)
    ET.ElementTree(icon).write(destination / f"term_{name}.xml", encoding="UTF-8")

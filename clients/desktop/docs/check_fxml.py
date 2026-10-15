#!/usr/bin/env python3
"""
Verificador de FXML de Sonimbus (cliente de escritorio).

Uso (desde la raíz del repositorio):
    python3 clients/desktop/docs/check_fxml.py            # revisa todos los FXML
    python3 clients/desktop/docs/check_fxml.py chat voice # solo esas carpetas

Revisa, por cada FXML bajo resources/mx/uv/sonimbus/desktop/ui/:
  - XML bien formado y xmlns="http://javafx.com/javafx/21"
  - imports explícitos (sin .*) y que cada elemento usado esté importado
  - sin estilos en línea (style="...") ni posiciones absolutas (layoutX/layoutY)
  - fx:id únicos; labelFor="$x" apunta a un fx:id declarado ANTES en el documento
  - botones sin texto tienen accessibleText
  - el controlador existe y tiene un campo @FXML por cada fx:id
    y un método por cada manejador onXxx="#metodo"
Sale con código 1 si hay errores.
"""
import os
import re
import sys
import xml.etree.ElementTree as ET

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "app", "src", "main"))
UI_RES = os.path.join(ROOT, "resources", "mx", "uv", "sonimbus", "desktop", "ui")
JAVA = os.path.join(ROOT, "java")

FX_NS = "http://javafx.com/fxml/1"
JFX_NS = "http://javafx.com/javafx/21"
FX = "{%s}" % FX_NS
BUTTONS = {"Button", "ToggleButton", "MenuButton", "SplitMenuButton"}


def local(tag):
    return tag.split("}", 1)[1] if "}" in tag else tag


def check(path):
    errors, warnings = [], []
    with open(path, encoding="utf-8") as fh:
        src = fh.read()

    m = re.search(r'<([A-Za-z][\w.]*)\s[^>]*?xmlns="([^"]+)"', src)
    if not m or m.group(2) != JFX_NS:
        errors.append('xmlns por defecto debe ser "%s"' % JFX_NS)

    imports = re.findall(r"<\?import\s+([\w.*]+)\s*\?>", src)
    simple = set()
    for imp in imports:
        if imp.endswith(".*"):
            errors.append("import con comodín: %s" % imp)
        simple.add(imp.rsplit(".", 1)[-1])

    try:
        tree = ET.fromstring(src)
    except ET.ParseError as e:
        return ["XML mal formado: %s" % e], warnings

    ids_seen, ids_order = set(), []
    handlers = []
    controller = tree.get(FX + "controller")

    for el in tree.iter():
        name = local(el.tag)
        if el.tag.startswith(FX):
            continue
        if name[:1].isupper() and "." not in name and name not in simple:
            errors.append("elemento <%s> sin <?import ...?>" % name)

        for k, v in el.attrib.items():
            key = local(k)
            if key == "style":
                errors.append("estilo en línea en <%s>: usa styleClass" % name)
            if key in ("layoutX", "layoutY"):
                errors.append("posición absoluta (%s) en <%s>" % (key, name))
            if key.startswith("on") and v.startswith("#"):
                handlers.append(v[1:])

        fxid = el.get(FX + "id")
        if fxid:
            if fxid in ids_seen:
                errors.append("fx:id duplicado: %s" % fxid)
            ids_seen.add(fxid)
            ids_order.append(fxid)

        lf = el.get("labelFor")
        if lf and lf.startswith("$") and lf[1:] not in ids_seen:
            errors.append('labelFor="%s" apunta a un fx:id no declarado antes' % lf)

        if name in BUTTONS and not el.get("text") and not el.get("accessibleText"):
            errors.append("<%s fx:id=%s> sin text ni accessibleText" % (name, fxid))

    if not controller:
        warnings.append("sin fx:controller")
        return errors, warnings

    jpath = os.path.join(JAVA, *controller.split(".")) + ".java"
    if not os.path.exists(jpath):
        errors.append("no existe el controlador %s" % os.path.relpath(jpath, ROOT))
        return errors, warnings

    with open(jpath, encoding="utf-8") as fh:
        jsrc = fh.read()
    fields = set(re.findall(r"@FXML\s+(?:private|protected|public)?\s*[\w<>?,\s\[\].]+?\s+(\w+)\s*;", jsrc))
    methods = set(re.findall(r"void\s+(\w+)\s*\(", jsrc))

    for fxid in ids_order:
        if fxid not in fields:
            errors.append("fx:id '%s' sin campo @FXML en %s" % (fxid, os.path.basename(jpath)))
    for h in handlers:
        if h not in methods:
            errors.append("manejador '#%s' sin método en %s" % (h, os.path.basename(jpath)))
    for f in fields - set(ids_order):
        warnings.append("campo @FXML '%s' no aparece como fx:id" % f)
    if "initialize" not in methods:
        warnings.append("el controlador no tiene initialize()")
    return errors, warnings


def main():
    only = set(sys.argv[1:])
    total_err = 0
    files = []
    for dirpath, _, names in os.walk(UI_RES):
        for n in sorted(names):
            if n.endswith(".fxml"):
                rel = os.path.relpath(os.path.join(dirpath, n), UI_RES)
                if not only or rel.split(os.sep)[0] in only:
                    files.append(rel)
    if not files:
        print("No se encontraron FXML en", UI_RES)
        return 1
    for rel in sorted(files):
        errs, warns = check(os.path.join(UI_RES, rel))
        status = "OK " if not errs else "ERR"
        print("[%s] %s" % (status, rel))
        for e in errs:
            print("      error: " + e)
        for w in warns:
            print("      aviso: " + w)
        total_err += len(errs)
    print("\n%d archivo(s), %d error(es)" % (len(files), total_err))
    return 1 if total_err else 0


if __name__ == "__main__":
    sys.exit(main())

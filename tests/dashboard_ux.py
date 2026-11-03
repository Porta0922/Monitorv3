"""Focused browser regression using synthetic API responses; no pilot credentials/data.
Build dashboard/dist, then python tests/dashboard_ux.py.
The browser intercepts the built static assets and API; no running server required.
Requires Python Playwright and its Chromium browser.
"""
import json
import mimetypes
from pathlib import Path
from collections import Counter
from datetime import datetime, timezone
from urllib.parse import urlparse, parse_qs
from playwright.sync_api import sync_playwright, expect

BASE = "http://monitor-ux.test"
DIST = Path(__file__).resolve().parents[1] / "dashboard" / "dist"
DAY = datetime.now(timezone.utc).date().isoformat()
STAMP = DAY + "T10:00:00Z"
DEVICE = {"device_id":"ux-device", "hostname":"Equipo UX", "mac_address":"", "last_seen":STAMP, "online":True, "created_at":STAMP}
COUNTERS = dict(available_samples=2, unavailable_samples=1, unknown_samples=0, active_person_seconds=120, idle_person_seconds=None, keys_count=None, mouse_moves_count=3, clicks_count=0)

def run():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        context = browser.new_context(viewport={"width":1440,"height":1000}, reduced_motion="reduce")
        page = context.new_page()
        exceptions, calls = [], Counter()
        pending=[]
        state = {"usb_error":True, "history_error":False, "role":"operator", "telemetry_full":False}
        page.on("pageerror", lambda e: exceptions.append(str(e)))
        def route(request):
            url = urlparse(request.request.url)
            path, query = url.path, parse_qs(url.query)
            if not path.startswith('/api/'):
                asset = (DIST / path.lstrip('/')).resolve()
                if not asset.is_relative_to(DIST.resolve()):
                    request.fulfill(status=404); return
                if not asset.is_file(): asset = DIST / 'index.html'
                request.fulfill(status=200,content_type=mimetypes.guess_type(asset.name)[0] or 'application/octet-stream',body=asset.read_bytes()); return
            calls[path] += 1
            status = 200
            if path == "/api/auth/session": data = dict(username="ux",role=state["role"],csrf_token="synthetic",expires_in=3600)
            elif path == "/api/devices": data = {"devices":[DEVICE]}
            elif path == "/api/devices/search": data = dict(devices=[DEVICE],total=1,page=1,page_size=20,counts=dict(total=1,online=1,offline=0,stale=0),groups=[])
            elif path.endswith("/telemetry"):
                data = {"resources":None,"health":None}
                if state["telemetry_full"]:
                    data["health"] = dict(stale=False,observed_at=STAMP,received_at=STAMP,data=dict(status="healthy",version="0.1.0",tasks=[dict(name=f"task-{i:02}",running=True,restart_count=0) for i in range(60)]))
            elif path.endswith("/sessions"): data = dict(day=DAY,timezone="UTC",total=0,totals=COUNTERS,items=[])
            elif path == "/api/usb/ux-device":
                if state.get("hold_usb"):
                    pending.append(request); return
                status = 503 if state["usb_error"] else 200
                data = {"events":[dict(timestamp=STAMP,device_id="ux-device",action="IN",hardware_id=f"usb-{i}",device_name=f"Unidad {i:02}",serial_number=str(i)) for i in range(60)]}
            elif path == "/api/wifi/ux-device": data = {"events":[dict(timestamp=query.get("date",[DAY])[0]+"T10:00:00Z",device_id="ux-device",interface_name="WiFi",state="connected",ssid="Oficina",signal_percent=70)]}
            elif path == "/api/history_hourly_programs":
                status = 503 if state["history_error"] else 200
                data = {"success":True,"groups":[dict(hour=10,app_name="EXCEL.EXE",window_title="Informe UX",total_seconds=120,count=1)]}
            elif path == "/api/live_devices": data = {"success":True,"devices":[]}
            elif path.endswith("/daily"): data = dict(from_=DAY,to=DAY,days=[]); data["from"]=DAY
            elif path.startswith("/api/resources/"): data = {"metrics":[]}
            elif path.startswith("/api/inventory/current/"): data = dict(sources=[],items=[],total=0)
            elif path.startswith("/api/inventory/"): data = {"apps":[]}
            elif path == "/api/export/csv": status,data = 403,{"error":"forbidden"}
            elif path == "/api/alerts": data = {"alerts":[]}
            elif path == "/api/resources_peaks": data = {"peaks":[]}
            else: data = {}
            request.fulfill(status=status,content_type="application/json",body=json.dumps(data))
        page.route("**/*", route)
        print("Checking equipment detail",flush=True)
        page.goto(f"{BASE}/devices/ux-device?date={DAY}&equipmentQuery=q%3Doficina")
        expect(page.get_by_role("heading",name="Equipo UX",exact=True)).to_be_visible()
        expect(page.get_by_role("heading",name="El equipo de un vistazo")).to_be_visible()
        expect(page.locator("html")).to_have_attribute("data-theme","dark")
        expect(page.get_by_text("Teclas: Sin datos")).to_be_visible()
        page.get_by_role("button",name="USB",exact=True).click()
        expect(page.get_by_text("No se pudo completar la consulta")).to_be_visible()
        before = calls["/api/history_hourly_programs"]
        state["usb_error"] = False
        page.get_by_role("button",name="Reintentar",exact=True).click()
        expect(page.get_by_role("row")).to_have_count(26)
        assert calls["/api/history_hourly_programs"] == before, "Retry must be local to USB"
        page.get_by_role("button",name="Siguiente",exact=True).click()
        expect(page.get_by_text("Página 2 de 3")).to_be_visible()
        page.get_by_role("searchbox").fill("Unidad 59")
        expect(page.get_by_role("row")).to_have_count(2)
        expect(page.get_by_text("Página 1 de 1")).to_be_visible()
        page.reload()
        expect(page.get_by_role("searchbox")).to_have_value("Unidad 59")
        expect(page.get_by_role("row")).to_have_count(2)
        state["usb_error"] = True
        page.get_by_role("button",name="Actualizar",exact=True).click()
        expect(page.get_by_text("Se muestran datos de la consulta anterior.",exact=False)).to_be_visible()
        expect(page.get_by_role("row")).to_have_count(2)
        page.get_by_label("Fecha de consulta (UTC)").fill("2026-01-01")
        expect(page.get_by_text("No se pudo completar la consulta")).to_be_visible()
        expect(page.get_by_role("row")).to_have_count(0)
        page.get_by_role("button",name="WiFi",exact=True).click()
        expect(page.get_by_text("Sin cierre observado")).to_be_visible()
        page.get_by_role("button",name="Actividad",exact=True).click()
        expect(page.get_by_role("heading",name="¿En qué se trabajó?")).to_be_visible()
        page.get_by_label("Buscar aplicación o título de ventana").fill("Informe UX")
        page.get_by_role("button",name="Resumen",exact=True).click()
        page.get_by_role("button",name="Actividad",exact=True).click()
        expect(page.get_by_label("Buscar aplicación o título de ventana")).to_have_value("Informe UX")
        page.get_by_role("button",name="Exportar actividad",exact=True).click()
        expect(page.get_by_text("No fue posible exportar la actividad.",exact=False)).to_be_visible()
        expect(page.get_by_role("heading",name="¿En qué se trabajó?")).to_be_visible()
        expect(page.get_by_text("El servidor rechazó una operación.",exact=False)).to_be_visible()
        page.get_by_role("button",name="Volver a los equipos").click()
        expect(page).to_have_url(BASE+"/dashboard?q=oficina")
        page.get_by_role("link",name="Ver detalle").click()
        expect(page).to_have_url(__import__('re').compile('equipmentQuery=q%3Doficina'))
        print("Checking responsive themes",flush=True)
        for width in (1440,768,390):
            page.set_viewport_size({"width":width,"height":900})
            for theme in ("light","dark"):
                page.get_by_label("Tema del dashboard").select_option(theme)
                expect(page.locator("html")).to_have_attribute("data-theme",theme)
                ratios=page.evaluate(r"""() => {
                  const node=document.createElement('span');document.body.append(node);
                  const rgb=token=>{node.style.color=`var(${token})`;return getComputedStyle(node).color.match(/[\d.]+/g).slice(0,3).map(Number);};
                  const lum=rgb=>rgb.map(v=>v/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4).reduce((sum,v,i)=>sum+v*[.2126,.7152,.0722][i],0);
                  const background=lum(rgb('--surface'));const result={};
                  for(const token of ['--ink','--muted','--accent']){const color=lum(rgb(token));result[token]=(Math.max(color,background)+.05)/(Math.min(color,background)+.05);}
                  node.remove();return result;
                }""")
                assert min(ratios.values())>=4.5, f"Text token contrast {theme}: {ratios}"
                artifacts=Path(__file__).resolve().parents[1]/'.local-test'/'ux-review'
                artifacts.mkdir(parents=True,exist_ok=True)
                page.screenshot(path=str(artifacts/f'{width}-{theme}.png'),full_page=True)
                assert page.evaluate("document.documentElement.scrollWidth <= innerWidth + 1"), f"Horizontal overflow at {width}/{theme}"
        state["telemetry_full"]=True
        page.get_by_role("button",name="Diagnóstico",exact=True).click()
        page.get_by_text("Diagnóstico de tareas",exact=True).click()
        expect(page.get_by_role("heading",name="Tareas del agente")).to_be_visible()
        task_section=page.locator("section").filter(has=page.get_by_role("heading",name="Tareas del agente")).last
        expect(task_section.get_by_role("row")).to_have_count(26)
        task_section.get_by_role("searchbox").fill("task-59")
        expect(task_section.get_by_role("row")).to_have_count(2)
        page.get_by_role("link",name="Ayuda",exact=True).click()
        expect(page.get_by_role("heading",name="Ayuda y operación diaria")).to_be_visible()
        menu=page.get_by_role("button",name="Abrir navegación")
        menu.focus()
        page.keyboard.press("Enter")
        expect(page.get_by_role("dialog",name="Menú de navegación")).to_be_visible()
        page.keyboard.press("Escape")
        expect(page.get_by_role("dialog",name="Menú de navegación")).not_to_be_visible()
        expect(menu).to_be_focused()
        page.keyboard.press("Tab")
        assert page.evaluate("document.activeElement.tagName") != "BODY"
        state["role"]="viewer"
        page.goto(BASE+"/devices/ux-device")
        expect(page.get_by_role("button",name="Exportar actividad",exact=True)).to_be_disabled()
        state["hold_usb"]=True
        page.goto(BASE+"/devices/ux-device?tab=usb")
        expect(page.get_by_text("Cargando registros…",exact=True)).to_be_visible()
        expect(page.get_by_role("heading",name="Equipo UX",exact=True)).to_be_visible()
        expect(page.get_by_text("Todavía no hay registros",exact=True)).not_to_be_visible()
        assert pending
        for held in pending: held.fulfill(status=200,content_type="application/json",body='{"events":[]}')
        expect(page.get_by_text("Todavía no hay registros",exact=True)).to_be_visible()
        assert not exceptions, exceptions
        print(json.dumps({"result":"passed","coverage":["partial_failure","local_retry","pagination_search","url_reload","stale_same_scope","no_cross_day_data","wifi_no_invented_duration","export_failure","permissions","return_filters","themes_responsive","help_keyboard","diagnostic_pagination","mobile_menu_focus","text_token_contrast","slow_section_loading","empty_after_success"],"page_errors":exceptions}))
        browser.close()

if __name__ == "__main__": run()

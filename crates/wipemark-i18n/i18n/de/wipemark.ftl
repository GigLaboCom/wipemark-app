### Wipemark — deutscher Katalog.
###
### Die Schlüssel stammen aus `en-US/wipemark.ftl`; dort steht auch, was
### beim Übersetzen zu beachten ist. Ein Schlüssel, den es dort nicht
### gibt, lässt die Testsuite scheitern.

-brand-name = Wipemark
-layer-a = Bereinigung
-layer-b = Umschreiben

## Fenster

window-title = { -brand-name }

## Die drei Bereiche

toolbar-import = Importieren…
toolbar-import-tooltip = Dateien oder Ordner auswählen. Sie kommen an wie beim Ablegen.
toolbar-import-choose = Importieren
toolbar-paste = Einfügen
toolbar-paste-text = Text einfügen
toolbar-paste-image = Bild einfügen
toolbar-paste-files = { $count ->
        [one] Datei einfügen
       *[other] { $count } Dateien einfügen
    }
toolbar-paste-items = { $count } Einträge einfügen
toolbar-paste-tooltip = Was in der Zwischenablage liegt, kommt an wie beim Ablegen. Ausgegraut, solange nichts darin ist, was dieses Fenster nehmen kann.
toolbar-help = Hilfe
toolbar-help-tooltip = Was dieses Fenster tut
toolbar-help-drop = Text, ein Bild oder Dateien irgendwo in diesem Fenster ablegen, mit Importieren auswählen – oder aus der Zwischenablage einfügen.
toolbar-help-preview = Den Zeiger auf einer Vorschau ruhen lassen, um sie größer zu sehen. Das Aktionen-Menü am Ende einer Zeile öffnet eine Datei mit der App, die das System nehmen würde.
toolbar-help-pending = „Bereinigen“ im Menü „Aktionen“ bereinigt eine Zeile, „Alles bereinigen“ jede wartende; das Ergebnis kommt dorthin, wo die Seite „Aufbewahrung“ es sagt. Umschreiben mit einem Modell gibt es in den Fenstern dieser Version noch nicht: Es läuft über die Kommandozeile (wipemark-cli rewrite) und, für einen Agenten, über MCP.
toolbar-help-elsewhere = Das Schnellreinigungs-Panel sitzt in der Menüleiste; die Einstellungen liegen hinter dem Zahnrad unten rechts.

queue-column-preview = Vorschau
queue-column-id = ID
queue-column-keyword = Stichwort
queue-column-name = Name
queue-column-kind = Art
queue-column-format = Format
queue-column-size = Größe
queue-column-arrived = Angekommen
queue-column-actions = Aktionen
queue-empty-invite = Noch nichts hier. Text, ein Bild oder Dateien ablegen, Importieren drücken – oder aus der Zwischenablage einfügen.
queue-empty-release = Loslassen, und es sagt, was es ist.
queue-count = { $count ->
        [one] { $count } Eintrag
       *[other] { $count } Einträge
    }
queue-pending = Umschreiben mit einem Modell gibt es in den Fenstern dieser Version noch nicht: Diese Liste bereinigt, und eine Umschreibung läuft über die Kommandozeile (wipemark-cli rewrite) und über MCP.
queue-preview-pending = Wird gelesen…
queue-preview-cut = Die ersten { $count } Zeichen; der Rest wird hier nicht gezeigt.
queue-actions = Aktionen
queue-action-open = Mit der Standard-App öffnen
queue-action-compare = Mit dem Ergebnis vergleichen
queue-copy = Kopieren
queue-filter-id = Nach ID suchen
queue-filter-keyword = Nach Stichwort suchen
queue-reset-filters = Filter zurücksetzen
queue-count-filtered = { $count } von { $total ->
        [one] { $total } Eintrag
       *[other] { $total } Einträgen
    }
queue-empty-filtered = Nichts passt zu diesen Filtern.
queue-sort-newest = Neueste zuerst
queue-sort-oldest = Älteste zuerst
queue-page-size = { $count } pro Seite
queue-page = Seite { $page } von { $pages }
queue-previous = Zurück
queue-next = Weiter

## Statuszeile

status-idle-no-engine = Bereit · keine Engine konfiguriert · nur { -layer-a }
status-idle-here = Bereit · { $model } · nichts verlässt diesen Rechner · nur { -layer-a }
status-idle-away = Bereit · { $model } auf { $host } · das Dokument verließe diesen Rechner · nur { -layer-a }
status-local-loading = { $model } wird geladen · nichts verlässt diesen Rechner · nur { -layer-a }
status-local-loading-progress = { $model } wird geladen — { $percent } % · nichts verlässt diesen Rechner · nur { -layer-a }
status-local-loaded = { $model } geladen · { -brand-name } belegt { $ram } · nichts verlässt diesen Rechner · nur { -layer-a }
status-local-loaded-unmeasured = { $model } geladen · nichts verlässt diesen Rechner · nur { -layer-a }
status-local-failed = { $model } konnte nicht geladen werden: { $reason } · nur { -layer-a }

## Einstellungen

settings-title = Einstellungen
settings-open = Einstellungen…
settings-appearance-title = Erscheinungsbild
settings-appearance-description = System folgt dem Schreibtisch, auch wenn er sich ändert, während { -brand-name } geöffnet ist.

settings-language-title = Sprache
settings-language-description = Jede Sprache steht unter ihrem eigenen Namen. System folgt dem Schreibtisch und wird beim Start von { -brand-name } aufgelöst.

settings-setup-title = Einrichtung
settings-setup-description = Der Rundgang, der sich beim ersten Start öffnet: wofür der Speicher dieses Rechners reicht, wer umschreibt, und das Modell oder die Adresse, die das braucht. Er ändert dieselben Zeilen wie die Seiten „Engine“ und „Modelle“ und sonst nichts.
settings-setup-run = Noch einmal durchgehen…
settings-setup-reset = Vergessen, dass er gezeigt wurde (Debug-Build)

## Einrichtung — der Rundgang.

setup-title = { -brand-name } einrichten
setup-skip = Überspringen
setup-back = Zurück
setup-next = Weiter
setup-finish = Fertig

setup-step-welcome = Willkommen
setup-step-machine = Dieser Rechner
setup-step-who = Wer umschreibt
setup-step-model = Das Modell
setup-step-endpoint = Die Adresse
setup-step-done = Fertig

setup-welcome-body = { -brand-name } entfernt KI-Herkunftsspuren aus Ihren eigenen Inhalten in zwei Schichten: { -layer-a }, die die unsichtbaren Zeichen entfernt und deterministisch ist, und { -layer-b }, das ein Sprachmodell um eine Umformulierung bittet. In dieser Version läuft die { -layer-a } aus diesen Fenstern — mit „Bereinigen“ im Hauptfenster und im Panel — sowie über die Kommandozeile und für einen Agenten über MCP; { -layer-b } läuft über die Kommandozeile und über MCP, aber noch nicht aus diesen Fenstern. Diese Schritte klären, was das Umschreiben braucht: wer umschreibt, und was dafür nötig ist.
setup-welcome-again = Alles hier lässt sich später in den Einstellungen ändern, und dieser Rundgang lässt sich von deren Seite „Allgemein“ aus wiederholen.

setup-machine-reading = Lese diesen Rechner…
setup-machine-here = Dieser Rechner meldet { $total } MB Arbeitsspeicher und hat Platz für { $model }, das etwa { $ram } MB braucht. Das Umschreiben kann auf diesem Rechner bleiben, und das ist die Empfehlung.
setup-machine-tight = Dieser Rechner meldet { $total } MB Arbeitsspeicher und fasst { $model }, das etwa { $ram } MB braucht — mit wenig Rest für alles andere. Das Umschreiben hier zu behalten bleibt die Empfehlung; eine Adresse ist die Alternative.
setup-machine-away = Dieser Rechner meldet { $total } MB Arbeitsspeicher, und nichts aus dem Katalog passt: dem kleinsten Modell, { $model }, fehlen { $short } MB. Eine Adresse — ein Server anderswo — ist die Empfehlung, und das Dokument ginge dorthin.
setup-machine-unjudged = Der Speicher dieses Rechners ließ sich nicht lesen, also wird nichts daran gemessen und nichts empfohlen. Im nächsten Schritt geht beides; { $model } ist das kleinste Modell im Katalog.
setup-machine-nothing = Der Katalog liefert kein Modell zum Umschreiben, also ist eine Adresse der einzige Weg zu einem.
setup-machine-unified = Ein gemeinsamer Speicher, den sich das Modell mit allem anderen teilt, was gerade läuft.
setup-machine-vram = Grafikspeicher: { $vram } MB. Er entscheidet, wie schnell, nicht ob ein Modell läuft.
setup-machine-vram-unknown = Grafikspeicher: nicht gemessen. Er entscheidet, wie schnell, nicht ob ein Modell läuft, also wird seinetwegen nichts abgelehnt.

setup-who-body = Zwei Dinge können ein Dokument umschreiben, und sie unterscheiden sich in dem einen Punkt, der zählt: ob das Dokument diesen Rechner verlässt.
setup-who-machine-line = Das Dokument verlässt diesen Rechner nie. Braucht ein geladenes Modell.
setup-who-endpoint-line = Ein Server, den Sie benennen. Das Dokument wird dorthin geschickt — und eine entfernte Adresse braucht zudem „{ settings-engine-allow-remote-title }“ auf der Seite „Engine“, bevor irgendetwas geschickt wird.
setup-who-ordered = Erst den einen und dann den anderen zu fragen ist ebenfalls eine Wahl, auf der Seite „Engine“.

setup-model-body = Gewichte werden einmal geladen, vor dem Behalten gegen die Prüfsumme des Katalogs geprüft und bleiben im Datenordner von { -brand-name }, bis sie entfernt werden. Das erste Modell, das ankommt, wird zum Umschreiben gewählt.
setup-model-others = Der übrige Katalog steht auf der Seite „Modelle“.
setup-open-models = Seite „Modelle“ öffnen…

setup-endpoint-body = Anbieter, Adresse, Modellname und Schlüssel sind die Zeilen der Seite „Engine“, und dieser Schritt öffnet sie, statt sie zu wiederholen. Kommen Sie hierher zurück, und die Zeile unten sagt, was diese Zeilen ergeben.
setup-open-engine = Seite „Engine“ öffnen…

setup-done-body = Alles hier Gewählte steht auf den Seiten „Engine“ und „Modelle“, und dieser Rundgang unter Einstellungen › Allgemein.

settings-shortcut-panel-title = Tastenkürzel für das Panel
settings-shortcut-panel-description = Ruft das Panel über das, woran Sie gerade arbeiten, und schickt es wieder weg. Dieses kommt mit einem bereits gesetzten Kürzel: andere Tasten aufnehmen, um es zu ändern; Rücktaste löscht es dauerhaft, Escape lässt es, wie es war.

settings-shortcut-show-title = Tastenkürzel, das { -brand-name } nach vorn holt
settings-shortcut-show-description = Wirkt aus jeder Anwendung, solange { -brand-name } läuft — auch, während das Fenster hinter dem Menüleisten-Symbol verborgen ist. Ins Feld klicken und die Tasten drücken; eine Zusatztaste außer Umschalt ist nötig. Rücktaste löscht es, Escape lässt es, wie es war.

hotkey-placeholder = Zum Festlegen klicken
hotkey-recording = Tasten drücken…
hotkey-needs-modifier = Zusätzlich eine Zusatztaste außer Umschalt gedrückt halten.
hotkey-unrecordable = Diese Taste kann nicht Teil eines Tastenkürzels sein.
hotkey-taken = Wird bereits von „{ $action }“ verwendet.

hotkey-registered = Aktiv aus jeder Anwendung, solange { -brand-name } läuft.
hotkey-refused = Das System hat es nicht angenommen: { $reason }
hotkey-unavailable = Gespeichert, aber nicht aktiv: systemweite Tastenkürzel gibt es auf dieser Plattform noch nicht.

## Bereiche der Seitenleiste

settings-section-general = Allgemein
settings-section-placement = Platzierung
settings-section-compare = Vergleich
settings-section-engine = Engine
settings-section-mcp = MCP
settings-section-retention = Aufbewahrung

## Platzierung der Fenster

## Das Panel — das Fenster, das man ruft.

panel-title = Schnelle Reinigung
panel-pending = Umschreiben mit einem Modell gibt es in den Fenstern dieser Version noch nicht: Es läuft über die Kommandozeile (wipemark-cli rewrite) und über MCP.
panel-dismiss = Escape schickt es wieder weg.

panel-help = Was hier möglich ist
panel-help-move = Ziehen Sie das Panel an einer beliebigen Stelle, um es zu verschieben.
panel-help-resize = Ziehen Sie an einer Kante oder Ecke, um die Größe zu ändern.
panel-help-dismiss = Escape schickt es weg; die Menüleiste und sein eigener Kurzbefehl holen es zurück.
panel-help-placement = Wo es aufgeht, wird pro Bildschirm gemerkt — in den Einstellungen unter „Platzierung“; und es von Hand zu verschieben sticht alles, was dort gewählt ist.

## Was auf dem Panel landet und was es sich als sein herausstellt.

panel-drop-invite = Text, ein Bild oder Dateien hierher ziehen.
panel-drop-release = Loslassen — dann steht hier, was es ist.
panel-drop-nothing = Bei diesem Ablegen kam nichts an, was sich lesen ließe.
panel-drop-more = … und { $count ->
        [one] ein weiteres
       *[other] { $count } weitere
    }

panel-drop-by-name = Nach dem Namen geurteilt — der Inhalt sagt weder das eine noch das andere.
panel-drop-mismatch = Der Name verspricht { $named }, der Inhalt ist { $found }.

panel-drop-result-beside = Das Ergebnis käme daneben, als { $name }; die Datei selbst bliebe unberührt.
panel-drop-result-into-file = Das Ergebnis käme nach { $folder }; die Datei selbst bliebe unberührt.
panel-drop-result-into = Das Ergebnis käme nach { $folder }.
panel-drop-result-over = Das Ergebnis träte an seine Stelle, sobald das Original als { $name } beiseitegelegt wäre.
panel-drop-result-as-text = Das Ergebnis käme als Text zurück; keine Datei würde geschrieben.
panel-drop-each-file = Jede Datei darin würde behandelt wie eine abgelegte Datei.
panel-drop-kept-originals = Eine Kopie des Originals bliebe { $period } erhalten.
panel-drop-kept-results = Eine Kopie des Ergebnisses bliebe { $period } erhalten.
panel-drop-kept-both = Kopien des Originals und des Ergebnisses blieben { $period } erhalten.

kind-text = Text
kind-image = Bild
kind-document = Dokument
kind-archive = Archiv
kind-media = Ton oder Video
kind-data = Daten
kind-folder = Ordner
kind-unknown = Nicht erkannt

## Das Vergleichsfenster — das Ergebnis neben seinem Original.

compare-title = Vergleich · { $name }
compare-original = Original
compare-result = Ergebnis
compare-pending = Im Vergleichsfenster ist das Ergebnis das, was die Bereinigung aus dem Original macht, und jede Zeile, die abweicht, ist auf beiden Seiten markiert. Änderungen am Ergebnis werden dort nicht gespeichert, und beim Schließen des Fensters wird nichts geschrieben.
compare-reading = Wird gelesen…
compare-same = Das Ergebnis ist das Original, Zeile für Zeile.
compare-changed = { $added ->
        [one] { $added } Zeile hinzugefügt
       *[other] { $added } Zeilen hinzugefügt
    }, { $removed ->
        [one] { $removed } Zeile entfernt
       *[other] { $removed } Zeilen entfernt
    }
compare-refused-not-text = Das ist kein Text, also gibt es nichts, was sich Zeile für Zeile vergleichen ließe.
compare-refused-too-big = Mit { $size } ist es mehr, als dieses Fenster vergleicht; die Grenze liegt bei { $limit }.
compare-refused-unreadable = Es ließ sich nicht lesen.
compare-reset = Zurück zum bereinigten Text
compare-reset-tooltip = Die Änderungen verwerfen; das Ergebnis ist wieder das, was die Bereinigung aus dem Original gemacht hat.
compare-help = Was dieses Fenster tut
compare-help-marks = Eine rote Markierung am Original ist eine Zeile, die das Ergebnis nicht mehr hat; eine grüne am Ergebnis eine Zeile, die das Original nie hatte.
compare-help-follows = Das Original folgt dem Cursor im Ergebnis, damit beide Seiten im Gleichschritt bleiben.
compare-help-toolbar = Die Leiste über dem Ergebnis sind die Befehle des Editors selbst, mit den Kurzbefehlen, auf die er ohnehin hört.
compare-help-words = Innerhalb einer geänderten Passage werden die Wörter, die abweichen, stärker markiert.
compare-help-characters = Innerhalb einer geänderten Passage werden die Zeichen, die abweichen, stärker markiert.
compare-help-settings = Was markiert wird und ob das Original folgt, wird auf der Seite „Vergleich“ der Einstellungen gewählt — für das nächste Fenster, das aufgeht.
compare-help-close = Wird dieses Fenster geschlossen, wird nichts geschrieben; Änderungen am Ergebnis leben nur hier.

result-undo = Rückgängig
result-redo = Wiederholen
result-cut = Ausschneiden
result-copy = Kopieren
result-paste = Einfügen
result-select-all = Alles auswählen
result-indent = Einrücken
result-outdent = Ausrücken
result-find = Suchen und ersetzen
result-soft-wrap = Lange Zeilen umbrechen
result-whitespace = Leerzeichen anzeigen


settings-compare-title = Wie ein Ergebnis verglichen wird
settings-compare-description = Was das Vergleichsfenster markiert, wenn ein Ergebnis neben sein Original gestellt wird. Ein Fenster liest diese Einstellungen beim Öffnen; ein bereits offenes behält, womit es geöffnet wurde.
settings-compare-exact = Jedes Zeichen zählt: der Vergleich übersieht kein Leerzeichen, kein Zeilenende und kein Zeichen, das man nicht sehen kann.

settings-compare-grain-title = Was markiert wird
settings-compare-grain-description = Jede Zeile, die abweicht, wird auf beiden Seiten markiert, was auch immer hier gewählt ist. Innerhalb einer Passage, die sich geändert hat — statt nur hinzugekommen oder weggefallen zu sein — können die Markierungen feiner werden: die Wörter, die abweichen, oder die einzelnen Zeichen.
settings-compare-grain-lines = Nur Zeilen
settings-compare-grain-words = Geänderte Wörter
settings-compare-grain-characters = Geänderte Zeichen

settings-compare-follow-title = Das Original folgt dem Cursor
settings-compare-follow-description = Wird der Cursor im Ergebnis bewegt, rollt das Original zu der Zeile, die an derselben Stelle steht, damit beide Seiten im Gleichschritt bleiben. Ausgeschaltet rollt jede Seite für sich.

settings-placement-title = Wo Fenster aufgehen
settings-placement-description = Auf welchem Bildschirm ein { -brand-name }-Fenster aufgeht, und an welcher Stelle dieses Bildschirms.

settings-placement-looking = Die Bildschirme werden gelesen…
settings-placement-attached = { $count ->
        [one] Ein Bildschirm angeschlossen.
       *[other] { $count } Bildschirme angeschlossen.
    }
settings-placement-only-window = Diese Einstellungen platzieren das { -brand-name }-Panel — das Fenster, das man aus der Menüleiste ruft. Die Fenster der Arbeitsfläche gehen nach denselben Regeln auf, sobald es sie gibt; das Hauptfenster und dieses hier platzieren sie nie.

settings-placement-close-title = Nach dem Ablegen schließen
settings-placement-close-description = Wird das Fenster auf einen Bildschirmteil gezogen, schließt sich dieses Fenster, damit sichtbar wird, wo es gelandet ist. Ein Klick auf einen Teil lässt dieses Fenster offen.

settings-placement-screen-title = Öffnen auf
settings-placement-screen-description = Der aktive Bildschirm ist der, auf dem der Zeiger steht, wenn das Fenster aufgeht. Der primäre ist der, auf dem der Schreibtisch die Menüleiste zeigt, wo der Zeiger auch sein mag.
settings-placement-screen-active = Der aktive Bildschirm
settings-placement-screen-primary = Der primäre Bildschirm

settings-placement-display = Bildschirm { $number }
settings-placement-resolution = { $width } × { $height }
settings-placement-primary = Primär
settings-placement-here = Das Panel ist hier
settings-placement-opens-in = Ein Fenster geht { $zone } auf diesem Bildschirm auf.
settings-placement-opens-where-left = Dort, wo Sie es auf diesem Bildschirm hingestellt haben, in der Größe, die Sie ihm gegeben haben.
settings-placement-where-it-was-left = Wo ich es hingestellt habe
settings-placement-restore-default = Standard wiederherstellen
settings-placement-drag-hint = Ziehen Sie es auf einen Teil des Bildschirms, oder klicken Sie einen an. Das Panel selbst zu verschieben oder zu vergrößern sticht beides.

settings-placement-zone-top-left = oben links
settings-placement-zone-top-centre = oben in der Mitte
settings-placement-zone-top-right = oben rechts
settings-placement-zone-bottom-left = unten links
settings-placement-zone-bottom-centre = unten in der Mitte
settings-placement-zone-bottom-right = unten rechts

## Engine

settings-engine-title = Umschreib-Engine
settings-engine-description = Umschreiben schickt das Dokument an ein Modell und bewertet, was zurückkommt. Die Bereinigung braucht dafür nie eine Engine und wird auch nie hinter einer verriegelt.

settings-engine-pending = Diese Fenster schreiben noch nichts um. Ein Agent kann es, über das Werkzeug rewrite des MCP-Servers, und wipemark-cli rewrite ebenso, solange diese Anwendung läuft — dann geht das Dokument dorthin, wen diese Seite zuständig macht: hierher oder an den Endpunkt. Die einzige Anfrage dieser Seite selbst ist die Prüfung unten, und sie schickt einen festen Satz.

settings-engine-state-off = Keine Engine. { -brand-name } bereinigt nur, was für sich genommen deterministisch und vollständig ist.
settings-engine-state-ready-local = Konfiguriert, und das Dokument bliebe auf diesem Rechner: { $endpoint }
settings-engine-state-ready-machine = Konfiguriert, und das Dokument verließe diesen Rechner nicht: { $model } läuft hier.
settings-engine-state-model-not-here = Das zum Umschreiben gewählte Modell liegt noch nicht auf diesem Rechner. Laden Sie es auf der Seite „Modelle“ herunter, oder richten Sie diese Seite auf einen Server.
settings-engine-state-model-unusable = Das zum Umschreiben gewählte Modell { $model } kann diese Version dafür nicht verwenden. Wählen Sie auf der Seite „Modelle“ ein anderes.
settings-engine-state-no-such-profile = Kein Profil namens „{ $name }“. Es wurde nichts an seiner Stelle verwendet.
settings-engine-state-ready-remote = Konfiguriert. Das Dokument ginge an { $endpoint } — nicht an diesen Rechner.
settings-engine-state-no-model = Kein Modell benannt. Jede Anfrage muss sagen, welches Modell sie beantwortet.
settings-engine-state-no-model-chosen = Auf diesem Rechner ist kein Modell zum Umschreiben gewählt. Gewählt wird eines auf der Seite „Modelle“, sobald es geladen ist.
settings-engine-state-remote-refused = { $host } ist nicht dieser Rechner, und Dokumente von ihm fortzuschicken wurde nicht erlaubt. Schalten Sie unten „{ settings-engine-allow-remote-title }“ ein, oder richten Sie die Adresse wieder auf diesen Rechner.
settings-engine-state-no-key = Für { $origin } ist kein Schlüssel hinterlegt. Dieser Anbieter braucht einen.
settings-engine-state-key-in-the-clear = { $origin } ist eine unverschlüsselte Verbindung zu einem anderen Rechner, der Schlüssel ginge also im Klartext über das Netz. { -brand-name } schickt ihn nicht. Nehmen Sie https oder eine Adresse auf diesem Rechner.
settings-engine-state-key-unreadable = Der Schlüssel ließ sich nicht lesen: { $reason }
settings-engine-state-checking = Suche nach einem hinterlegten Schlüssel…

settings-engine-serves-title = Wer umschreibt
settings-engine-serves-description = Zwei Dinge können ein Dokument umschreiben: ein auf der Seite „Modelle“ geladenes Modell, das diesen Rechner nie verlässt, und die Adresse unten, also ein Server irgendwo. Eine Reihenfolge wird angesagt und nicht verborgen — antwortet die zweite Wahl, sagt der Hinweis oben, welche und warum — und eine entfernte Adresse braucht weiterhin „{ settings-engine-allow-remote-title }“, bevor irgendetwas verschickt wird.
settings-engine-serves-machine = Dieser Rechner
settings-engine-serves-endpoint = Die Adresse
settings-engine-serves-machine-first = Rechner, dann Adresse
settings-engine-serves-endpoint-first = Adresse, dann Rechner
settings-engine-state-second-choice = Antwortet, weil die erste Wahl es nicht kann: { $reason }

## Das Modell auf diesem Rechner.

settings-engine-keep-title = Modell geladen halten
settings-engine-keep-description = Für das Modell auf diesem Rechner; ein Endpunkt hält hier nichts. „{ settings-engine-keep-on-demand }“ lädt das Modell, wenn es gebraucht wird, und gibt seinen Speicher nach den Minuten unten ohne Arbeit wieder frei. „{ settings-engine-keep-resident }“ lädt es kurz nach dem Start von { -brand-name } und hält es, bis { -brand-name } beendet oder ein anderes Modell gewählt wird. „{ settings-engine-local-unload }“ gibt es in beiden Fällen frei.
settings-engine-keep-on-demand = Bei Bedarf laden
settings-engine-keep-resident = Geladen halten
settings-engine-idle-title = Entladen nach
settings-engine-idle-description = Wie lange ein bei Bedarf geladenes Modell ohne Arbeit im Speicher bleibt. Ohne Wirkung, solange das Modell geladen gehalten wird.
settings-engine-idle-minutes = { $count ->
        [one] { $count } Minute
       *[other] { $count } Minuten
    }
settings-engine-advanced = Erweitert
settings-engine-lock-title = Modell im Arbeitsspeicher halten (nicht auslagern lassen)
settings-engine-lock-description = Die erste Anfrage nach einer Pause wird dann nicht dadurch gebremst, dass das Modell von der Platte zurückgelesen wird; der Preis ist dieser Speicher, den kein anderes Programm leihen kann, solange das Modell geladen ist. Verweigert das System die Sperre, wird das Modell trotzdem geladen und das Protokoll sagt es.

settings-engine-local-title = Das Modell auf diesem Rechner
settings-engine-local-not-here = Was gerade zuständig ist, läuft nicht auf diesem Rechner; hier gibt es kein Modell zu laden.
settings-engine-local-not-loaded = { $model } ist nicht geladen.
settings-engine-local-resident-again = „{ settings-engine-keep-resident }“ ist weiterhin gewählt, also wird es beim nächsten Start von { -brand-name } wieder geladen.
settings-engine-local-loading = { $model } wird geladen…
settings-engine-local-loading-progress = { $model } wird geladen — { $percent } % gelesen…
settings-engine-local-loaded = { $model } ist geladen. { -brand-name } belegt { $ram } Speicher, gemessen. Geladen um { $since }.
settings-engine-local-loaded-unmeasured = { $model } ist geladen, seit { $since }. Wie viel Speicher es belegt, ließ sich nicht lesen.
settings-engine-local-file = Die Modelldatei belegt { $size } auf der Platte.
settings-engine-local-failed = Konnte nicht geladen werden: { $reason }
settings-engine-local-unload = Jetzt entladen
settings-engine-local-unload-tooltip = Den Speicher des Modells jetzt freigeben. Die nächste Anfrage lädt es wieder.
settings-engine-local-unload-disabled = Es ist kein Modell geladen, es gibt nichts zu entladen.
settings-engine-local-check = Prüfen
settings-engine-local-check-tooltip = Das Modell laden, falls es nicht geladen ist, und ein paar Wörter schreiben lassen.
settings-engine-local-check-cancel = Abbrechen
settings-engine-local-checking = Wird geprüft…
settings-engine-local-check-answered = Das Modell antwortete: „{ $text }“
settings-engine-local-check-load = Das Laden dauerte { $seconds } s.
settings-engine-local-check-speed = { $tokens } Tokens, { $rate } pro Sekunde nach dem ersten.
settings-engine-local-check-speed-unknown = { $tokens } Tokens — zu wenige, um sie zu messen.
settings-engine-local-check-failed = Die Prüfung lief nicht: { $reason }
settings-engine-local-check-cancelled = Die Prüfung wurde abgebrochen.
settings-engine-local-check-note = Eine Prüfung zeigt, dass das Modell lädt und schreibt. Sie ist keine Umschreibung: Diese Fenster schreiben noch nichts um, ein Agent über MCP und die Kommandozeile schreiben mit diesem Modell um.
settings-engine-local-no-tray = Auf diesem System gibt es keinen Eintrag in der Menüleiste; das Schließen des Hauptfensters beendet { -brand-name } also und gibt das Modell frei.

settings-engine-remote-title = Der Endpunkt
settings-engine-remote-asks = Eine Prüfung fragt { $model } unter { $endpoint }.
settings-engine-remote-refused = Der Endpunkt kann nicht gefragt werden: { $reason }
settings-engine-remote-check-tooltip = Einen festen Satz an den Endpunkt schicken und zeigen, was er zurückschreibt.
settings-engine-remote-check-answered = Der Endpunkt antwortete: „{ $text }“
settings-engine-remote-check-first = Das erste Stück kam nach { $seconds } s.
settings-engine-remote-check-speed = { $pieces } Stücke, { $rate } pro Sekunde nach dem ersten.
settings-engine-remote-check-speed-unknown = { $pieces } Stücke — zu wenige, um sie zu messen.
settings-engine-remote-check-note = Eine Prüfung zeigt, dass der Endpunkt antwortet. Sie ist keine Umschreibung: Diese Fenster schreiben noch nichts um, ein Agent über MCP und die Kommandozeile schicken ihre Dokumente hierher.
settings-engine-remote-check-sent-to = Ihre Eingabe — ein fester Satz, nie ein Dokument — geht an { $origin }, und das ist nicht dieser Rechner.

engine-refusal-not-built = Dieser Build hat keine lokale Engine.
engine-refusal-no-such-file = Die Modelldatei ist nicht da: { $path }
engine-refusal-would-not-fit = Das Modell braucht etwa { $need }, und dieser Rechner hat { $have }.
engine-refusal-no-backend = Es wurde kein Prozessor gefunden, auf dem das Modell laufen könnte.
engine-refusal-load-failed = Das Modell konnte nicht geladen werden.
engine-refusal-stopped = Die lokale Engine wurde beendet. Das Modell erneut zu wählen startet sie neu.
engine-refusal-nothing-on-duty = Es ist nichts zuständig, das antworten könnte.
engine-refusal-redirected = Der Endpunkt antwortete mit { $status } und verwies auf { $origin }. Weiterleitungen folgt { -brand-name } nicht: Korrigieren Sie stattdessen die Adresse.
engine-refusal-redirected-nowhere = Der Endpunkt antwortete mit { $status }, einer Weiterleitung. Weiterleitungen folgt { -brand-name } nicht: Korrigieren Sie stattdessen die Adresse.
engine-refusal-key-rejected = Der Endpunkt hat den Schlüssel nicht angenommen ({ $status }).
engine-refusal-not-found = Der Endpunkt kennt dieses Modell oder diese Adresse nicht. Prüfen Sie den Namen des Modells — bei Ollama auch, ob es mit „pull“ geholt wurde.
engine-refusal-rate-limited-for = Der Endpunkt begrenzt die Anfragen und bittet, { $seconds } s zu warten.
engine-refusal-rate-limited = Der Endpunkt begrenzt die Anfragen. Versuchen Sie es später noch einmal.
engine-refusal-refused = Der Endpunkt hat die Anfrage abgelehnt ({ $status }).
engine-refusal-key-unreadable = Der Schlüssel ließ sich nicht aus dem Anmeldeinformationsspeicher lesen: { $reason }
engine-refusal-no-key = Für diesen Endpunkt ist kein Schlüssel hinterlegt.
engine-refusal-key-unsendable-empty = Der für diesen Endpunkt hinterlegte Schlüssel ist leer, und es wurde nichts gesendet. Bitte den Schlüssel auf der Seite „Engine“ erneut speichern.
engine-refusal-key-unsendable-not-ascii = Der für diesen Endpunkt hinterlegte Schlüssel enthält ein Zeichen, das kein reines ASCII ist und das eine Anfrage nicht tragen kann; es wurde nichts gesendet. Bitte den Schlüssel auf der Seite „Engine“ erneut speichern.
engine-refusal-key-unsendable-control = Der für diesen Endpunkt hinterlegte Schlüssel enthält ein Steuerzeichen, das eine Anfrage nicht tragen kann; es wurde nichts gesendet. Bitte den Schlüssel auf der Seite „Engine“ erneut speichern.
engine-refusal-key-unsendable-space = Der für diesen Endpunkt hinterlegte Schlüssel enthält ein Leerzeichen, das eine Anfrage nicht tragen kann; es wurde nichts gesendet. Bitte den Schlüssel auf der Seite „Engine“ erneut speichern.

settings-engine-profile-title = Gespeichertes Profil
settings-engine-profile-description = Alle Einstellungen dieser Seite außer dem Schlüssel, unter einem Namen abgelegt. Ein Profil auszuwählen übernimmt alles auf einmal, und unter einem schon vergebenen Namen zu speichern ersetzt es. Der Schlüssel bleibt im Anmeldeinformationsspeicher dieses Rechners, abgelegt unter der Adresse, und gilt für jedes Profil, das dorthin zeigt.
settings-engine-profile-placeholder = Gespeichertes Profil wählen
settings-engine-profile-name-placeholder = Diesen Einstellungen einen Namen geben
settings-engine-profile-save = Speichern…
settings-engine-profile-delete = Löschen
settings-engine-profile-saved = Gespeichert als „{ $name }“.
settings-engine-profile-modified = „{ $name }“, mit ungespeicherten Änderungen.
settings-engine-profile-unsaved = Unter keinem Namen gespeichert.
settings-engine-profile-no-key = Der Schlüssel gehört nicht zum Profil.

settings-engine-profile-name-title = Diese Einstellungen speichern
settings-engine-profile-name-body = Unter einem neuen Namen oder unter einem, den Sie schon verwenden.
settings-engine-profile-name-taken = Eines davon ersetzen:
settings-engine-profile-name-confirm = Speichern
settings-engine-profile-delete-title = „{ $name }“ löschen?
settings-engine-profile-delete-body = Nur die gespeicherte Kopie verschwindet. Die Einstellungen auf dieser Seite bleiben genau so, wie sie sind, und der Schlüssel im Anmeldeinformationsspeicher dieses Rechners ebenso.
settings-engine-profile-delete-confirm = Löschen
settings-engine-profile-cancel = Abbrechen

settings-engine-provider-title = Anbieter
settings-engine-provider-description = Ollama spricht sein eigenes /api/chat; der andere erreicht alles, was /v1/chat/completions bedient. Ohne Engine läuft { -layer-a } für sich weiter.
settings-engine-provider-off = Keine Engine
settings-engine-provider-openai = OpenAI-kompatibel

settings-engine-endpoint-title = Adresse
settings-engine-endpoint-description = Die Basis-URL ohne Pfad — den passenden hängt { -brand-name } selbst an. Nur http und https, und eine URL mit Benutzernamen oder Passwort darin wird abgelehnt.

settings-engine-model-title = Modell
settings-engine-model-description = Der Name, unter dem die Gegenstelle das Modell kennt, genau so geschrieben — llama3.1:8b, gpt-4o-mini, deepseek/deepseek-chat.

settings-engine-key-title = API-Schlüssel
settings-engine-key-description = Liegt im Anmeldeinformationsspeicher dieses Rechners, abgelegt unter der Adresse, für die er eingegeben wurde, und nie in den eigenen Einstellungen von { -brand-name }. Nach dem Speichern wird er nicht wieder angezeigt.
settings-engine-key-placeholder = Schlüssel einfügen, um ihn zu hinterlegen
settings-engine-key-save = Speichern
settings-engine-key-forget = Verwerfen
settings-engine-key-stored = Für { $origin } ist ein Schlüssel hinterlegt.
settings-engine-key-absent = Für { $origin } ist kein Schlüssel hinterlegt.
settings-engine-key-not-used = Dieser Anbieter schickt keinen Schlüssel. Ollamas eigene API kennt keinen Authorization-Header.
settings-engine-key-would-be-in-the-clear = { $origin } ist unverschlüsselt und nicht dieser Rechner. Ein dort hinterlegter Schlüssel ginge nur im Klartext hinaus, also schickt { -brand-name } keinen.
settings-engine-key-failed = Der Anmeldeinformationsspeicher hat abgelehnt: { $reason }
settings-engine-key-refused-not-ascii = Nicht gespeichert: Dieser Schlüssel enthält ein Zeichen, das kein reines ASCII ist — einen Buchstaben einer anderen Tastaturbelegung, einen typografischen Strich oder ein unsichtbares Zeichen aus dem Einfügen —, und eine Anfrage kann es nicht tragen. Bitte den Schlüssel erneut einfügen.
settings-engine-key-refused-control = Nicht gespeichert: Dieser Schlüssel enthält ein Steuerzeichen, das eine Anfrage nicht tragen kann. Bitte den Schlüssel erneut einfügen.
settings-engine-key-refused-space = Nicht gespeichert: Dieser Schlüssel enthält ein Leerzeichen, und kein Anbieter vergibt Schlüssel mit einem. Bitte den Schlüssel erneut einfügen.
settings-engine-key-refused-empty = Nicht gespeichert: Im Feld steht kein Schlüssel.
settings-engine-key-not-persistent = Dieser Rechner hat keinen Anmeldeinformationsspeicher, den { -brand-name } erreicht; ein hier eingegebener Schlüssel hält nur, bis die Anwendung schließt.

settings-engine-allow-remote-title = Entfernte Adresse erlauben
settings-engine-allow-remote-description = Aus muss die Adresse dieser Rechner sein. An geht der Text jedes Dokuments an den, der sie betreibt — genau der Sinn eines gehosteten Modells, und eine Entscheidung, die man trifft statt in sie hineinzurutschen.

settings-engine-temperature-title = Temperatur
settings-engine-temperature-description = Zwischen 0 und 2. Höher entfernt sich weiter vom Wortlaut, was der Sinn des Umschreibens ist und zugleich der Weg, auf dem eine Tatsache verloren geht.

settings-engine-reasoning-title = Denkaufwand
settings-engine-reasoning-description = In einer Paraphrase steckt kein Schlussfolgern. Wird als „none“ geschickt; „off“ lässt das Feld ganz weg, für Server, die den Wert ablehnen statt ihn zu übergehen.
settings-engine-reasoning-off = Aus (weglassen)
settings-engine-reasoning-none = Keiner
settings-engine-reasoning-low = Gering
settings-engine-reasoning-medium = Mittel
settings-engine-reasoning-high = Hoch

settings-engine-timeout-title = Zeitlimit
settings-engine-timeout-description = Sekunden, die auf eine Antwort gewartet wird, bevor sie aufgegeben wird. Ein Modell, das denkt, kann für einen Satz Minuten brauchen.

## MCP

settings-section-models = Modelle
settings-models-title = Lokale Modelle
settings-models-description = Offene Gewichte, die auf dieses Gerät geladen und gegen die Prüfsumme im Katalog von { -brand-name } geprüft werden. Nichts wird geladen, bevor Sie es verlangen.
settings-models-pending = Ein heruntergeladenes Modell kann auf der Seite „Engine“ geladen und geprüft werden; diese Fenster schreiben damit noch nichts um, ein Agent über MCP und wipemark-cli rewrite schon. Die Bereinigung braucht davon nichts.
settings-models-folder-title = Modellordner
settings-models-folder-description = Wohin Downloads gelegt werden und wo { -brand-name } nach Modelldateien sucht — in diesem Ordner und in jedem Ordner darunter. Ein geleertes Feld stellt den Standard wieder her.
settings-models-folder-choose = Auswählen…
settings-models-folder-default = Standard
settings-models-folder-busy = Warten Sie, bis der Download beendet ist, bevor Sie den Ordner verschieben.
settings-models-folder-missing = Modellordner: { $path } — existiert noch nicht; der erste Download legt ihn an.
settings-models-folder-unreadable = Modellordner: { $path } — konnte nicht gelesen werden: { $reason }
settings-models-folder-read = Modellordner: { $path } — { $installed ->
        [one] ein Katalogmodell hier
       *[other] { $installed } Katalogmodelle hier
    }, { $other ->
        [0] sonst nichts, das wie ein Modell aussieht
        [one] eine weitere Modelldatei
       *[other] { $other } weitere Modelldateien
    }.
settings-models-found-title = Außerdem in diesem Ordner
settings-models-found-description = Modelldateien, die beim Durchsuchen des Ordners und aller Unterordner gefunden wurden. Sie stehen nicht im Katalog dieser Version, also kann hier nichts sie prüfen, und noch nichts setzt sie ein.
settings-models-rewrite-title = Modell zum Umschreiben
settings-models-rewrite-description = Welches geladene Modell ein Umschreiben verwenden würde. Aufgeführt sind nur Modelle, die bereits auf diesem Gerät liegen; ein Modell wird für einen Zweck gewählt, und Umschreiben ist der einzige Zweck, für den dieser Build Gewichte mitbringt.
settings-models-rewrite-none = Kein lokales Modell
settings-models-size = { $size } Download
settings-models-needs = Braucht etwa { $ram } MB
settings-models-download = Laden
settings-models-resume = Fortsetzen
settings-models-cancel = Anhalten
settings-models-remove = Entfernen
settings-models-installed = Auf diesem Gerät
settings-models-progress = { $done } von { $total }
settings-models-verifying = Prüfe, was schon da ist …
settings-models-checking = Prüfe { $done } von { $total } gegen den Katalog …
settings-models-host = Dieses Gerät meldet { $ram } MB Arbeitsspeicher.
settings-models-host-unknown = Der Arbeitsspeicher dieses Geräts war nicht lesbar, daher wird unten nichts daran gemessen.
settings-models-fit-roomy = Dafür ist Platz, und der Rest des Geräts bleibt benutzbar.
settings-models-recommended = Für diesen Rechner empfohlen
settings-models-fit-tight = Würde passen, mit wenig Rest für alles andere.
settings-models-fit-too-big = { $short } MB mehr Arbeitsspeicher, als dieses Gerät hat.
settings-models-fit-unknown = Nicht beurteilt: Der Arbeitsspeicher dieses Geräts war nicht lesbar.
settings-models-damaged = Auf diesem Gerät, aber nicht das, was der Katalog beschreibt. Entfernen und neu laden.
settings-models-found-at = Gefunden unter { $path }. { -brand-name } hat diese Datei nicht heruntergeladen; sie wird dort verwendet, wo sie liegt, und nie entfernt.
settings-models-loading = Wird in den Speicher geladen — { $percent } % gelesen…
settings-models-failed = Der Download wurde beendet: { $reason }
settings-models-stopped = Angehalten. Das bereits Geladene bleibt erhalten, der nächste Versuch setzt darauf auf.

settings-retention-title = Was aufbewahrt wird
settings-retention-description = Wohin ein Ergebnis geht, was mit der Datei geschieht, aus der es stammt, und ob { -brand-name } eine Kopie von dem behält, was ohne Datei ankam.

settings-retention-beside = Ergebnisse werden neben die Datei geschrieben, als name.cleaned.ext; die Datei selbst wird nie angerührt.
settings-retention-into = Ergebnisse werden nach { $folder } geschrieben; die Datei selbst wird nie angerührt.
settings-retention-over = Eine Datei wird durch ihr Ergebnis ersetzt, sobald das Original als name.original.ext beiseitegelegt ist — und ein bereits vorhandenes Original wird nie überschrieben.
settings-retention-keeps-nothing = Was ohne Datei ankommt — Eingefügtes, aus einem Browser Gezogenes — wird nicht aufbewahrt, sobald das Ergebnis es ersetzt hat.
settings-retention-keeps-originals = Das Original von Eingefügtem oder Gezogenem bleibt in { $folder } { $period } erhalten, wenn das Bereinigen daran etwas geändert hat; Ergebnisse nicht.
settings-retention-keeps-results = Das Ergebnis von Eingefügtem oder Gezogenem bleibt in { $folder } { $period } erhalten; Originale nicht.
settings-retention-keeps-both = Original und Ergebnis von Eingefügtem oder Gezogenem bleiben in { $folder } { $period } erhalten, wenn das Bereinigen daran etwas geändert hat.
settings-retention-pending = Die Fenster bereinigen nach diesen Regeln: Was sie schreiben und was sie aufbewahren, legt diese Seite fest. Die Kommandozeile und Agenten lesen keine davon — der Kommandozeile wird bei jedem Aufruf gesagt, wohin ihr Ergebnis geht, und ein Agent bekommt sein Ergebnis zurück.

settings-retention-destination-title = Wohin Ergebnisse gehen
settings-retention-destination-description = „Neben die Datei“ schreibt name.cleaned.ext daneben und lässt die Datei, wie sie ist. Der Ergebnisordner ist der unten. „Anstelle der Datei“ ersetzt sie — nachdem das Original als name.original.ext beiseitegelegt wurde, und nie über ein bereits vorhandenes Original.
settings-retention-destination-beside = Neben die Datei
settings-retention-destination-folder = In den Ergebnisordner
settings-retention-destination-replace = Anstelle der Datei

settings-retention-folder-title = Ergebnisordner
settings-retention-folder-description = Wohin Ergebnisse gehen, wenn sie in einen Ordner gehen — und wohin ein Ergebnis geht, das neben keiner Datei liegen kann, etwa ein aus einem Browser gezogenes Bild. Ein leeres Feld stellt den Downloads-Ordner wieder her.

settings-retention-originals-title = Eingefügtes behalten
settings-retention-originals-description = Eingefügter Text und hineingezogene Bilder haben keine Datei hinter sich; sobald das Ergebnis sie ersetzt hat, ist das Original weg. Behält eine Kopie im eigenen Ordner von { -brand-name } für den Zeitraum unten, so wie sie ankam, samt Auszeichnung. Eine Datei wird nie hierher kopiert: Die Datei ist das Original.

settings-retention-results-title = Ergebnisse behalten
settings-retention-results-description = Das Ergebnis von Eingefügtem oder Gezogenem, für den Zeitraum unten im eigenen Ordner von { -brand-name } behalten, damit es erreichbar bleibt, wenn die Zwischenablage längst etwas anderes enthält. Ein in eine Datei geschriebenes Ergebnis wird nicht hierher kopiert.

settings-retention-period-title = Wie lange
settings-retention-period-description = Wie lange eine behaltene Kopie bleibt, bevor { -brand-name } sie entfernt. Außerhalb des eigenen Ordners von { -brand-name } wird dadurch nie etwas entfernt.
settings-retention-period-day = Einen Tag
settings-retention-period-week = Eine Woche
settings-retention-period-month = Einen Monat
settings-retention-period-quarter = Drei Monate
settings-retention-period-forever = Bis zum Löschen von Hand
settings-retention-span-day = einen Tag lang
settings-retention-span-week = eine Woche lang
settings-retention-span-month = einen Monat lang
settings-retention-span-quarter = drei Monate lang
settings-retention-span-forever = bis zum Löschen von Hand

settings-mcp-title = MCP-Server
settings-mcp-description = Ein Agent kann { -layer-a } über das Model Context Protocol auf seine eigene Ausgabe anwenden und mit der zuständigen Engine umschreiben.
settings-mcp-tools = Fünf Werkzeuge arbeiten: inspect zeigt, was die { -layer-a } an einem Text ändern würde, clean nimmt diese Änderungen vor und meldet jede mit ihrer Position, inspect_image und clean_image tun dasselbe mit einem PNG, JPEG oder WebP — mit seinen Metadaten und mit den sichtbaren Markierungen in seinen Pixeln, die diese Version kennt und die clean_image entfernt, wenn es sie belegen kann; Markierungen, die kein Auge sieht, werden weder gesucht noch entfernt —, und rewrite lässt die zuständige Engine den Text zwischen zwei Durchgängen der { -layer-a } umschreiben — das Dokument geht dorthin, wohin die Seite „Engine“ es schickt. Umschreiben geschieht nach bestem Bemühen, und sein Bericht sagt, was es nicht feststellt.

settings-mcp-status-off = Läuft nicht.
settings-mcp-status-starting = Startet…
settings-mcp-status-listening = Läuft und antwortet unter { $url }
settings-mcp-status-moved = Port { $wanted } war bereits belegt, daher wurde { $port } genommen. Das Snippet unten ist das passende.
settings-mcp-status-failed = Start fehlgeschlagen: { $reason }

settings-mcp-exposed = { $address } ist aus dem Netz erreichbar, und dieser Server verlangt kein Passwort. Alles, was diesen Rechner erreicht, kann { -layer-a } darauf ausführen.

settings-mcp-enabled-title = Über MCP anbieten
settings-mcp-enabled-description = Startet mit { -brand-name } und läuft, solange es läuft. Wird die Adresse oder der Port unten geändert, startet er neu.

settings-mcp-bind-title = Lauschen auf
settings-mcp-bind-description = Jede Adresse dieses Rechners — 127.0.0.1 antwortet nur ihm selbst, 0.0.0.0 antwortet allem, was ihn erreicht, und 192.168.1.101 antwortet nur auf dieser Schnittstelle.

settings-mcp-port-title = Port
settings-mcp-port-description = Zwischen 1024 und 65535. Ein bereits belegter Port wird übersprungen: der Server nimmt den nächsten freien und sagt, welchen.

settings-mcp-endpoint = Clients verbinden sich mit { $url }

settings-mcp-snippets-title = Client verbinden
settings-mcp-snippets-description = Dies in die Konfiguration des Clients einfügen. In einen vorhandenen mcpServers-Block einfügen, statt ihn zu ersetzen.
settings-mcp-copy = Kopieren
settings-mcp-copied = Kopiert

settings-mcp-client-generic = Beliebiger MCP-Client

## Erscheinungsbild

theme-system = System
theme-light = Hell
theme-dark = Dunkel

## Sprache

language-autonym = Deutsch
language-selector-label = Sprache
language-system = System ({ $language })

## Das dritte Regal (Spezifikation §0.1, Regel 3)

report-not-established-title = Nicht belegt
report-not-established-vendor-detector-evasion = Umgehung des herstellereigenen Detektors — nicht geprüft, ein Orakel dafür gibt es nicht
report-not-established-human-authorship = menschliche Urheberschaft — von keiner Prüfung dieses Werkzeugs belegt
report-not-established-unknown-mark-schemes = Markierungen in Verfahren, die dieser Build nicht umsetzt — es wurde nicht danach gesucht
report-not-established-invisible-pixel-marks = unsichtbare Markierungen in den Pixeln des Bildes — nicht gesucht, nicht entfernt

unicode-class-zero-width = breitenloses Zeichen
unicode-class-zwj = breitenloser Verbinder
unicode-class-bidi-control = Steuerzeichen der Schreibrichtung
unicode-class-tag-character = Tag-Zeichen
unicode-class-variation-selector = Variantenselektor
unicode-class-soft-hyphen = bedingter Trennstrich
unicode-class-exotic-space = ungewöhnliches Leerzeichen
unicode-class-noncharacter = Nichtzeichen
unicode-class-private-use = Zeichen für private Nutzung
unicode-class-default-ignorable = ignorierbares Formatzeichen
unicode-class-homoglyph = Buchstabe aus einer anderen Schrift

confidence-confirmed = bestätigt
confidence-probable = wahrscheinlich
confidence-informational = zur Information
confidence-likely-false-positive = wahrscheinlich keine Markierung

## Menüleiste

tray-show = { -brand-name } anzeigen
tray-panel = Panel anzeigen
tray-clean-clipboard = Zwischenablage säubern — noch nicht
tray-unload-model = Modell entladen
tray-appearance = Erscheinungsbild
tray-quit = { -brand-name } beenden

## Kommandozeile

cli-about = Entfernt KI-Herkunftsmarkierungen aus Ihren eigenen Texten und Bildern

cli-help-usage = Aufruf:
cli-help-commands = Befehle:
cli-help-arguments = Argumente:
cli-help-options = Optionen:
cli-help-print-help = Hilfe anzeigen
cli-help-print-version = Version anzeigen
cli-command-help = Diese Meldung anzeigen, oder die Hilfe zum angegebenen Befehl.

cli-command-inspect = Meldet, was in einem Dokument oder in einem PNG-, JPEG- oder WebP-Bild steckt — in seinen Metadaten, und ein sichtbares Zeichen in seinen Pixeln, das ein bekanntes Profil beschreibt —, ohne es zu ändern. Nach unsichtbaren Zeichen in den Pixeln wird nicht gesucht.
cli-command-clean = Nur { -layer-a }: deterministisch, nachprüfbar, ohne Modell. Ein PNG-, JPEG- oder WebP-Bild verliert seine Metadaten mit KI-Herkunft; ist das alles, bleiben seine Bilddaten Byte für Byte erhalten. Ein nachgewiesenes sichtbares Zeichen wird entfernt, und das Bild wird dann neu geschrieben — ein JPEG mit Qualität 95, ein verlustbehaftetes WebP verlustfrei. Unsichtbare Zeichen in den Pixeln bleiben.
cli-command-rewrite = { -layer-a }, dann ein Umschreiben durch das Modell, dann wieder { -layer-a }.
cli-command-models = Heruntergeladene Gewichte verwalten.
cli-command-models-list = Jedes Modell im Katalog auflisten, was davon auf diesem Rechner liegt und ob es passt.
cli-command-models-pull = Ein Modell anhand seiner Id herunterladen, eine Teildatei wird fortgesetzt.
cli-command-models-verify = Ein installiertes Modell vollständig neu hashen und mit dem Katalog vergleichen. Exit 1, wenn es nicht passt oder fehlt.
cli-command-models-rm = Ein installiertes Modell löschen.
cli-command-audit = Einen Ordner durchlaufen und jede Textdatei darin mit Funden melden, und jedes PNG, JPEG oder WebP, dessen Metadaten KI-Herkunft tragen oder dessen Pixel ein sichtbares Zeichen tragen, für Pre-Commit-Hooks und CI. Nach unsichtbaren Zeichen in den Pixeln wird nicht gesucht. Exit 3, sobald eine Datei nicht gelesen werden konnte — auch wenn andere Funde hatten: Ein Scan mit einer Lücke ist nicht vollständig.

cli-arg-path-or-stdin = Zu lesende Datei, oder `-` für die Standardeingabe.
cli-arg-path = Zu lesende Datei.
cli-arg-out = Ausgabedatei, oder `-` für die Standardausgabe. Standard ist `<name>.cleaned.<ext>` neben der Eingabe, und die Standardausgabe, wenn die Eingabe die Standardeingabe ist; direktes Überschreiben braucht ein ausdrückliches Flag und ist nie der Standard.
cli-arg-nfkc = NFKC-Normalisierung anwenden (standardmäßig aus — sie ändert mehr als nur Herkunftsmarkierungen).
cli-arg-aggressive = Auch einen Buchstaben aus einer anderen Schrift innerhalb eines Wortes ersetzen (ein Homoglyph). Gemeldet werden solche Buchstaben in jedem Fall; höhere Falsch-positiv-Rate, deshalb nur auf Wunsch.
cli-arg-json = Maschinenlesbares JSON statt Fließtext.
cli-arg-tactic = Wie das Modell gefragt wird: paraphrase (Standard), humanize oder back_translate. structural gibt es nur in der Anwendung, nach einer Bestätigung; code gibt es in dieser Version nicht.
cli-arg-candidates = Kandidaten je Abschnitt. Ohne Angabe entscheidet, wer umschreibt: 1 für ein Modell allein auf dem Prozessor dieses Rechners, 2 für eines auf einer Grafikkarte oder einen Endpunkt.
cli-arg-rounds = Höchstzahl der Durchgänge je Abschnitt. Ohne Angabe bis zu 2, und der zweite nur, wenn kein Kandidat des ersten bestanden hat.
cli-arg-intensity = Wie weit paraphrase und humanize sich vom Wortlaut entfernen dürfen: light, moderate (Standard) oder strong.
cli-arg-format = Was der Text ist: plain, markdown oder html. Ohne Angabe das, was die Datei ist; für alles andere plain. In markdown und html wird nur der Fließtext umgeschrieben.
cli-arg-prompts = Eine JSON-Datei mit Vorlagenzeilen über denen, die die Anwendung gespeichert hat — der Schlüssel einer Zeile und entweder ihr gespeicherter Wert oder der Text der Vorlage. Eine Vorlage, die eine Regel bricht, hält den Lauf an, bevor etwas gesendet wird.
cli-arg-seed = Der Basis-Seed. Ohne Angabe bekommt jeder Lauf einen neuen, und ein erneuter Lauf schreibt anders um; der Seed aus einem Bericht, zurückgegeben, wiederholt einen Lauf auf einem Modell dieses Rechners.
cli-arg-id = Modell-Id aus dem Manifest.
cli-arg-dir = Zu durchlaufendes Verzeichnis.
cli-arg-sarif = SARIF-Ausgabe für Code-Scanning-Dashboards.
cli-arg-in-place = Die Datei durch ihren bereinigten Text oder ihr bereinigtes Bild ersetzen. Das Original wird vorher daneben als `<name>.original.<ext>` beiseitegelegt, und ein bereits vorhandenes Original wird nie überschrieben: Der Lauf verweigert dann. Wenn nichts zu ändern ist, wird nichts angefasst.
cli-arg-all-metadata = Für ein Bild: jeden Metadatenblock entfernen, nicht nur die KI-Herkunft — auch Kameradaten (EXIF, mit der Ausrichtung, auf die sich ein Bild verlassen kann, um aufrecht zu erscheinen), XMP, IPTC, Kommentare. Farbprofile bleiben in jedem Fall erhalten: Ohne eines sieht das Bild anders aus. Nicht für Text.
cli-arg-no-original = Mit --in-place: keine Kopie des Originals behalten — für Dateien unter Versionsverwaltung, deren Historie die Kopie ist.
cli-arg-language = Sprache für Meldungen und Hilfe, als BCP-47-Tag wie de oder ru. Hat Vorrang vor WIPEMARK_LANG, der in den Einstellungen gespeicherten Sprache und dem Betriebssystem, in dieser Reihenfolge.

cli-report-stdin = Standardeingabe
cli-report-none = { $source }: Keines der Zeichen, nach denen diese Version sucht, wurde gefunden.
cli-report-noted = { $source }: { $count ->
        [one] ein Zeichen gefunden, und es ist wahrscheinlich keine Markierung.
       *[other] { $count } Zeichen gefunden, und keines davon ist wahrscheinlich eine Markierung.
    }
cli-report-suspicious = { $source }: { $count ->
        [one] ein Zeichen gefunden, und es sieht nach einer Markierung aus.
       *[other] { $count } Zeichen gefunden, und mindestens eines davon sieht nach einer Markierung aus.
    }
cli-report-would-remove = Würde entfernt:
cli-report-would-replace = Würde ersetzt:
cli-report-would-keep = Würde behalten:
cli-report-removed = Entfernt:
cli-report-replaced = Ersetzt:
cli-report-kept = Behalten:
cli-report-row = { $character } · { $class } · { $confidence } · { $count ->
        [one] einmal, bei Byte { $positions }
       *[other] { $count }-mal, bei den Bytes { $positions }
    }
cli-report-more = { $shown } und { $more } weitere
cli-report-homoglyphs-kept = Buchstaben aus einer anderen Schrift wurden gefunden und nicht ersetzt; clean ersetzt sie nur mit --aggressive, jeweils durch den gleich aussehenden Buchstaben der eigenen Schrift des Wortes.
cli-report-offsets = Die Byte-Positionen zählen den Text als UTF-8; die Eingabe war { $encoding }.
cli-report-unicode = Geprüft gegen Unicode { $version }.
cli-clean-written = Das Ergebnis steht in { $path }.
cli-clean-untouched = { $source } selbst wurde nicht verändert.
cli-clean-nfkc = Zusätzlich wurde die NFKC-Normalisierung angewendet; was sie freigelegt hat, wurde in weiteren Durchgängen bereinigt, und --json zählt es ohne Positionen.
cli-clean-later = { $count ->
        [one] Ein weiteres Zeichen, das NFKC freigelegt hat, wurde in einem weiteren Durchgang entfernt oder ersetzt; es hat keine Position in der Eingabe, daher kann eine Zeile oben es noch als behalten aufführen.
       *[other] { $count } weitere Zeichen, die NFKC freigelegt hat, wurden in weiteren Durchgängen entfernt oder ersetzt; sie haben keine Position in der Eingabe, daher kann eine Zeile oben sie noch als behalten aufführen.
    }
cli-no-such-file = { $path } existiert nicht.
cli-is-a-folder = { $path } ist ein Ordner. inspect und clean lesen eine Datei oder die Standardeingabe; audit durchläuft einen Ordner.
cli-out-is-a-folder = --out nennt einen Ordner, { $path }. Erwartet wird der Name einer Datei.
cli-out-is-input = --out nennt die Datei, die gelesen wird, { $path }. Um die Datei zu ersetzen, gibt es --in-place, das das Original vorher beiseitelegt.
cli-unreadable = { $path } konnte nicht gelesen werden: { $reason }. Nicht gelesen heißt nicht sauber.
cli-not-text = { $path }: Der Inhalt ist { $format }, kein Text, also hat die { -layer-a } hier nichts zu lesen. Nicht gelesen heißt nicht sauber.
cli-not-text-unknown = { $path } ist in keiner Kodierung Text, die diese Version liest. Nicht gelesen heißt nicht sauber.
cli-unnamed-encoding = { $path } ist Text in einer 8-Bit-Kodierung, die diese Version nicht benennt. Als UTF-8 speichern und erneut ausführen; bis dahin ist die Datei nicht gelesen, und nicht gelesen heißt nicht sauber.
cli-invalid-encoding = { $path } ist bei Byte { $offset } kein gültiges { $encoding }. Nicht gelesen heißt nicht sauber.
cli-name-disagrees = { $path }: Der Name verspricht { $named }, der Inhalt ist { $found }; gelesen wurde nach dem Inhalt.
cli-write-failed = { $path } konnte nicht geschrieben werden: { $reason }. Das Ergebnis wurde nicht gespeichert.

cli-in-place-stdin = --in-place ersetzt eine Datei, und die Standardeingabe ist keine. Eine Datei nennen oder das Ergebnis mit --out schreiben.
cli-in-place-link = { $path } ist ein symbolischer Link. --in-place ersetzt Dateien, keine Links; auf die Datei anwenden, auf die der Link zeigt.
cli-in-place-original-exists = { $original } existiert bereits, und ein früher beiseitegelegtes Original wird nie überschrieben. { $path } wurde nicht geändert. { $original } woandershin verschieben oder mit --no-original ausführen.
cli-in-place-set-aside-failed = { $path } konnte nicht als { $original } beiseitegelegt werden: { $reason }. Nichts wurde geändert.
cli-in-place-write-failed = Der bereinigte Text konnte nicht nach { $path } geschrieben werden: { $reason }. { $path } wurde nicht geändert.
cli-in-place-stranded = Der bereinigte Text konnte nicht nach { $path } geschrieben werden ({ $reason }), und das Original konnte nicht zurückgelegt werden ({ $restore }). Das Original liegt jetzt unter { $original }.
cli-in-place-original = Das Original wurde als { $original } beiseitegelegt.
cli-in-place-no-original = Keine Kopie des Originals wurde behalten (--no-original).
cli-in-place-unchanged = { $source }: Es war nichts zu ändern, die Datei wurde also nicht angefasst und kein Original beiseitegelegt.

cli-audit-not-a-folder = { $path } ist kein Ordner. audit durchläuft einen Ordner; inspect und clean lesen eine Datei.
cli-audit-file = { $path }: { $count ->
        [one] { $count } Fund
       *[other] { $count } Funde
    } ({ $classes })
cli-audit-summary = { $root }: gescannt { $scanned } · mit Funden { $findings } · übersprungen { $skipped } · nicht lesbar { $unreadable }
cli-audit-unreadable-title = Nicht lesbar, also nicht als sauber gezeigt:
cli-audit-image-visible = { $path }: { $count ->
    [one] eine sichtbare Markierung
   *[other] { $count } sichtbare Markierungen
} ({ $profiles })
cli-audit-image = { $path }: { $container }, { $count ->
        [one] ein Block
       *[other] { $count } Blöcke
    } mit KI-Herkunft ({ $kinds })

cli-image-none = { $source }: { $container }, keine Metadatenblöcke.
cli-image-noted = { $source }: { $container }, { $count ->
        [one] ein Metadatenblock, und er weist nicht auf KI-Herkunft hin.
       *[other] { $count } Metadatenblöcke, und keiner davon weist auf KI-Herkunft hin.
    }
cli-image-ai = { $source }: { $container }, { $count ->
        [one] ein Metadatenblock
       *[other] { $count } Metadatenblöcke
    }, { $ai ->
        [one] einer davon mit KI-Herkunft.
       *[other] { $ai } davon mit KI-Herkunft.
    }
cli-image-row = { $where } · { $kind } · ab Byte { $offset } · { $length ->
        [one] { $size } Byte
       *[other] { $size } Bytes
    }
cli-image-evidence = { $signal }, in { $field }: { $matched }
cli-image-evidence-generator = { $signal } ({ $generator }), in { $field }: { $matched }
cli-image-rendering = Farbinformationen (ein ICC-Profil, Gamma, sRGB) bleiben in jedem Fall erhalten: Ohne sie sähe das Bild anders aus.
cli-image-exif-removed = Ein EXIF-Block nannte einen Bildgenerator und wurde deshalb ganz entfernt, samt den Kameradaten darin.
cli-image-all-metadata = --all-metadata hat auch die Kameradaten entfernt.
cli-image-orientation-removed = Die Drehung des Bildes stand in den entfernten Kameradaten: Ein Programm, das es aufrecht gezeigt hat, zeigt es jetzt so, wie es gespeichert ist – gedreht oder gespiegelt.
cli-image-pixels = Die Pixel wurden auf die sichtbaren Markierungen geprüft, die diese Version kennt. Markierungen, die kein Auge sieht, werden nicht gesucht, und nichts hier sagt, dass das Bild keine trägt.
cli-image-visible-title = Sichtbare Markierungen
cli-image-visible-none = In den Pixeln wurde keine sichtbare Markierung gefunden, die diese Version kennt.
cli-image-visible-row = { $profile } ({ $vendor }, { $product }) · { $width }×{ $height } bei { $x },{ $y } · { $placed }
cli-image-visible-placed-row = an ihrer bekannten Stelle
cli-image-visible-placed-searched = durch Suchen gefunden
cli-image-visible-proved = belegt: Korrelation { $ncc }, Stärke { $gain }, Kantenanteil { $ratio }
cli-image-visible-refused = gesehen, nicht belegt: { $reason }
cli-image-refusal-transparent = das Bild ist unter der Markierung nicht deckend
cli-image-refusal-opaque = die Markierung ist überall deckend ({ $holes } Pixel), und darunter lässt sich nichts zurückgewinnen
cli-image-refusal-gain = ihre Kanten verschwinden bei einer Stärke von { $k }, nicht bei ihrer eigenen
cli-image-refusal-edges = nach dem Entfernen bliebe { $ratio } ihres Umrisses
cli-image-refusal-out-of-range = das Entfernen würde { $share } der Werte aus dem Wertebereich schieben
cli-image-visible-restored = { $profile }: { $changed } Pixel wiederhergestellt.
cli-image-visible-exact = Die wiederhergestellten Pixel sind die ursprünglichen Werte bis auf eine Stufe.
cli-image-visible-inexact = Das Bild war verlustbehaftet gespeichert; die Wiederherstellung ist so nah, wie die gespeicherten Werte es erlauben, aber nicht exakt.
cli-image-visible-holes = { $holes } Pixel unter einem deckenden Teil der Markierung ließen sich nicht zurückgewinnen und blieben, wie sie waren.
cli-image-visible-outline = Entlang ihres Randes ist ein Umriss der Markierung geblieben — im Mittel { $levels } Stufen vom Bild um sie herum im am stärksten abweichenden Farbkanal, { $share } % ihrer Kontur —, mehr als diese Version zulässt, daher gilt die Markierung als noch im Ergebnis.
cli-image-visible-texture = Entlang des Randes der Markierung ist eine Körnung geblieben — beim 95. Perzentil liegen ihre Pixel { $levels } Stufen von ihren Nachbarn, im Bild um sie herum { $around } —, mehr als diese Version zulässt, daher gilt die Markierung als noch im Ergebnis.
cli-image-visible-clamped = { $clamped ->
        [one] Beim Umkehren der Überblendung fiel { $clamped } Wert aus dem Wertebereich und wurde begrenzt, daher ist die Wiederherstellung nicht exakt.
       *[other] Beim Umkehren der Überblendung fielen { $clamped } Werte aus dem Wertebereich und wurden begrenzt, daher ist die Wiederherstellung nicht exakt.
    }
cli-image-visible-fitted = Die Deckkraftkarte der Markierung wurde an echten Ausgaben gemessen und nicht vom Hersteller übernommen, daher wird keine exakte Wiederherstellung behauptet.
cli-image-visible-resampled = Die Markierung stand nicht an der Stelle und in der Größe, für die ihre Karte gezeichnet ist; die Karte wurde umgerechnet, daher wird keine exakte Wiederherstellung behauptet.
cli-image-visible-searched = Die Markierung wurde von der Suche gefunden, nicht an der Stelle, die ihr Profil nennt, daher wird keine exakte Wiederherstellung behauptet.
cli-image-visible-residual = Entlang ihres schwachen Randes liegt die wiederhergestellte Markierung im Mittel { $levels } Stufen vom Bild um sie herum, im am stärksten abweichenden Farbkanal.
cli-image-visible-left = Eine sichtbare Markierung wurde gefunden und ist noch im Ergebnis.
cli-image-visible-not-restorable = Diese Art Bild (ein CMYK-JPEG) schreibt diese Version nicht zurück, daher blieb die Markierung.
cli-image-visible-not-examined-animated = Ein animiertes Bild: Seine Einzelbilder wurden nicht auf sichtbare Markierungen geprüft, nur seine Metadaten. Nicht geprüft ist nicht sauber.
cli-image-visible-not-examined-catalogue = Der Katalog sichtbarer Markierungen dieses Builds wurde nicht geladen, daher wurden die Pixel nicht geprüft. Nicht geprüft ist nicht sauber.
cli-image-visible-not-examined-decode = Die Pixel des Bildes ließen sich nicht dekodieren, daher wurden sie nicht auf sichtbare Markierungen geprüft. Nicht gelesen ist nicht sauber.
cli-image-encoded-jpeg = Das Bild wurde als JPEG mit Qualität { $quality } neu kodiert.
cli-image-encoded-webp = Das Bild wurde als verlustfreies WebP geschrieben.
cli-image-encoded-webp-from-lossy = Das Bild war verlustbehaftetes WebP und wurde als verlustfreies WebP geschrieben: Die Datei ist größer, und kein weiterer Verlust kam hinzu.
cli-image-encoded-png = Das PNG wurde mit den wiederhergestellten Pixeln neu geschrieben.
cli-image-encoded-png-colour = Der Farbtyp des PNG hat sich geändert: Die wiederhergestellten Farben passten nicht in den ursprünglichen.
cli-image-encoded-png-interlace = Das PNG wurde ohne Zeilensprung geschrieben.
cli-image-proof-failed = { $path }: Das Ergebnis hat seine eigene Prüfung nicht bestanden, daher wurde nichts geschrieben. Das ist ein Fehler dieser Version.
cli-image-encode-failed = { $path }: Das wiederhergestellte Bild ließ sich nicht zurückschreiben. Nichts wurde geschrieben.

cli-image-not-yet = { $path }: { $container }-Bilder sind in dieser Version noch nicht dabei. Es wurden keine Metadaten gelesen und nichts geschrieben.
cli-image-unknown = { $path }: Diese Bytes sind kein Bild, das diese Version öffnet. Nichts wurde geschrieben.
cli-image-multi-picture = { $path } enthält nach dem ersten Bild weitere (MPF), und dieses Entfernen würde deren Index falsch machen: Metadaten hinter dem Index würden sie verschieben, oder der Index ließ sich nicht lesen, um ihn zu berichtigen. Nichts wurde geschrieben.
cli-image-reframe = { $path }: Die Pixel des Bildes haben sich geändert, und diese Version kann sie nicht in diese Datei zurückschreiben: Sie ist animiert oder enthält einen Teil, den diese Version nicht kennt. Nichts wurde geschrieben.
cli-image-malformed = { $path } ist keine { $container }-Datei, die diese Version lesen kann: { $defect }, bei Byte { $offset }. Nicht gelesen heißt nicht sauber.
cli-image-text-flag = { $path } ist ein Bild ({ $container }), und { $flag } ist für Text. Nichts wurde geschrieben.
cli-image-all-metadata-text = { $path } ist kein Bild, und --all-metadata ist für Bilder. Nichts wurde geschrieben.
cli-image-to-terminal = Das bereinigte Bild ginge an ein Terminal. Mit -o in eine Datei schreiben oder die Standardausgabe umleiten.
cli-image-json-stdout = --json schreibt seine Antwort auf die Standardausgabe, und das Bild ginge ebenfalls dorthin; das Bild mit -o in eine Datei schreiben.
cli-image-still-marked = { $path }: Das Ergebnis trüge noch Metadaten mit KI-Herkunft, deshalb wurde es nicht geschrieben. Nicht jede Markierung ließ sich entfernen.

image-kind-c2pa = C2PA-Manifest
image-kind-exif = EXIF
image-kind-xmp = XMP
image-kind-iptc = IPTC
image-kind-generator-parameters = Generatorparameter
image-kind-other-text = Text
image-kind-rendering = Farbinformationen
image-kind-other = sonstige Metadaten

image-signal-c2pa-manifest = ein C2PA-Manifest
image-signal-c2pa-reference = ein Verweis auf ein C2PA-Manifest
image-signal-digital-source-type = ein IPTC-Digital-Source-Type, der ein Modell oder einen Algorithmus nennt
image-signal-generator-key = ein Textschlüssel, den ein Bildgenerator schreibt
image-signal-generator-text = die Signatur eines Bildgenerators

image-defect-truncated = sie endet mitten in einem Block
image-defect-bad-signature = ihre Signatur steht nicht an ihrem Platz
image-defect-header-not-first = ihr Kopf ist nicht der erste Block
image-defect-no-end = ihr fehlt die Endmarke
image-defect-bad-length = ein Block hat eine Länge, die kein Block haben kann
image-defect-bad-chunk-type = der Name eines Chunks besteht nicht aus vier Buchstaben
image-defect-bad-marker = wo eine Marke stehen muss, steht ein anderes Byte
image-defect-riff-size = ihr RIFF-Kopf gibt mehr Bytes an, als die Datei hat
image-defect-bad-text = ein Text-Chunk ist nicht wie einer aufgebaut
image-defect-inflate = ein komprimierter Text lässt sich nicht entpacken
image-defect-inflate-limit = ein komprimierter Text wird beim Entpacken größer als die Grenze, die diese Version liest

cli-models-folder = Modellordner: { $path }
cli-models-entry = { $id } · { $name } · { $roles } · { $size } · { $state } · { $fit }
cli-models-chosen = zum Umschreiben gewählt
cli-models-found-at = gefunden unter { $path }
cli-models-state-present = auf diesem Rechner, entspricht dem Katalog
cli-models-state-absent = nicht heruntergeladen
cli-models-state-partial = teilweise heruntergeladen ({ $percent } %), pull setzt fort
cli-models-state-mismatch = auf diesem Rechner, entspricht aber nicht dem Katalog
cli-models-fit-fits = passt auf diesen Rechner
cli-models-fit-tight = passt auf diesen Rechner, mit wenig Reserve
cli-models-fit-too-big = braucht { $short } MB mehr Speicher, als dieser Rechner hat
cli-models-fit-unknown = ob es auf diesen Rechner passt, ist unbekannt
cli-models-size = { $gigabytes } GB
cli-models-others-title = Außerdem in diesem Ordner, nicht im Katalog — nur aufgelistet, nicht geprüft, und nichts lädt sie:
cli-models-folder-unreadable = Der Modellordner { $path } konnte nicht gelesen werden: { $reason }.
cli-models-unknown-id = { $id } steht nicht im Katalog. Seine Ids sind: { $ids }.
cli-models-pull-present = { $id } liegt bereits auf diesem Rechner und entspricht dem Katalog: { $path }
cli-models-pull-progress = { $id }: { $done } von { $total } MB ({ $percent } %)
cli-models-pull-done = { $id } wurde heruntergeladen und entspricht dem Katalog: { $path }
cli-models-pull-cancelled = { $id }: abgebrochen. Das Heruntergeladene bleibt erhalten; pull erneut ausführen, um fortzusetzen.
cli-models-pull-mismatch = { $id }: { $file } entspricht nicht dem Katalog (erwartet sha256 { $expected }, erhalten { $actual }), wurde also verworfen, die Teildatei mit ihm. Nichts wurde installiert.
cli-models-pull-no-room = { $id } braucht { $need } MB auf dem Volume mit { $path }, und { $free } MB sind frei. Nichts wurde heruntergeladen.
cli-models-pull-failed = { $id } konnte nicht heruntergeladen werden: { $reason }. Das bisher Heruntergeladene bleibt erhalten; pull erneut ausführen, um fortzusetzen.
cli-models-verify-ok = { $id } entspricht dem Katalog: Jede Datei wurde vollständig gehasht.
cli-models-verify-absent = { $id } liegt nicht auf diesem Rechner ({ $file } fehlt), entspricht also nicht dem Katalog.
cli-models-verify-mismatch = { $id }: { $file } entspricht nicht dem Katalog (erwartet sha256 { $expected }, erhalten { $actual }). pull lädt die Datei erneut herunter.
cli-models-verify-unreadable = { $id }: { $file } konnte nicht gelesen werden: { $reason }. Nicht gelesen ist nicht geprüft.
cli-models-rm-removed = { $id } wurde aus { $path } entfernt.
cli-models-rm-absent = { $id } lag nicht auf diesem Rechner; nichts wurde entfernt.
cli-models-rm-found = { $id } liegt unter { $path }, wohin { -brand-name } es nicht heruntergeladen hat; nichts wurde entfernt.
cli-models-rm-chosen = Es war das zum Umschreiben gewählte Modell: Die Anwendung zeigt kein gewähltes Modell, bis ein anderes ausgewählt wird. Dieser Befehl ändert diese Einstellung nicht.
cli-models-rm-failed = { $id } konnte nicht aus { $path } entfernt werden: { $reason }.

cli-unknown-language = unbekannte Sprache `{ $requested }`, es wird zurückgefallen. Verfügbar: { $available }

cli-rewrite-tactic-structural = Die Taktik structural schreibt ein Dokument aus einer Gliederung neu und ist nur in der Anwendung zu haben, nach einer Bestätigung. Nichts wurde umgeschrieben.
cli-rewrite-tactic-code = Die Taktik code gibt es in dieser Version nicht. Nichts wurde umgeschrieben.
cli-rewrite-needs-app-endpoint = Das Umschreiben ist auf einen Endpunkt eingestellt, und die Kommandozeile erreicht einen nur über die laufende Anwendung { -brand-name }. Starten Sie sie und führen Sie dies erneut aus — oder lassen Sie auf ihrer Seite „Engine“ ein Modell auf diesem Rechner umschreiben. Nichts wurde umgeschrieben.
cli-rewrite-needs-app-fallback = Das zum Umschreiben gewählte Modell ist nicht auf diesem Rechner, und der Endpunkt, der stattdessen antworten soll, ist nur über die laufende Anwendung { -brand-name } erreichbar. Laden Sie das Modell herunter, oder starten Sie die Anwendung und führen Sie dies erneut aus. Nichts wurde umgeschrieben.
cli-rewrite-no-model = Es ist kein Modell auf diesem Rechner zum Umschreiben gewählt. Laden Sie eines mit wipemark-cli models pull und wählen Sie es auf der Seite „Modelle“ der Anwendung — oder starten Sie die Anwendung mit einem zuständigen Endpunkt. Nichts wurde umgeschrieben.
cli-rewrite-model-not-here = Das zum Umschreiben gewählte Modell, { $id }, ist nicht vollständig auf diesem Rechner; wipemark-cli models pull { $id } holt es. Nichts wurde umgeschrieben.
cli-rewrite-unavailable = Nichts wurde umgeschrieben: { $reason }
cli-rewrite-failed = Nichts wurde umgeschrieben: Der Auftrag ist fehlgeschlagen ({ $reason }).
cli-rewrite-cancelled = Abgebrochen. Nichts wurde geschrieben.
cli-rewrite-lost = Die Anwendung hat aufgehört zu antworten, bevor die Umschreibung zurückkam. Nichts wurde geschrieben; vielleicht beendet sie den Auftrag noch.
cli-rewrite-app-refused = Die laufende Anwendung hat nicht umgeschrieben: { $reason }
cli-rewrite-served-app = Umgeschrieben von der laufenden Anwendung { -brand-name }, mit ihrer zuständigen Engine.
cli-rewrite-served-here = Umgeschrieben von diesem Befehl, mit dem zum Umschreiben gewählten Modell.
cli-rewrite-price = { $calls ->
        [one] Das Modell wird höchstens einmal gefragt, für etwa { $tokens } Token.
       *[other] Das Modell wird höchstens { $calls }-mal gefragt — { $expected }-mal, wenn jeder Absatz sofort besteht — für etwa { $tokens } Token.
    }
cli-rewrite-progress = Absatz { $chunk } von { $chunks } · Kandidat { $candidate } von { $candidates } · Durchgang { $round } von { $rounds }
cli-rewrite-summary = { $chunks ->
        [0] Es enthielt keinen Fließtext zum Umschreiben; Code, Überschriften und Auszeichnung bleiben, wie sie sind.
        [one] { $rewritten } von einem Absatz wurde umgeschrieben.
       *[other] { $rewritten } von { $chunks } Absätzen wurden umgeschrieben.
    }
cli-rewrite-kept = { $kept ->
        [one] Ein Absatz behält sein bereinigtes Original: Kein Kandidat dafür hat die Prüfungen bestanden. Nicht alles wurde umgeschrieben, deshalb ist der Exit-Code 3.
       *[other] { $kept } Absätze behalten ihr bereinigtes Original: Kein Kandidat dafür hat die Prüfungen bestanden. Nicht alles wurde umgeschrieben, deshalb ist der Exit-Code 3.
    }
cli-rewrite-attempts = { $attempts ->
        [one] Das Modell schrieb einen Kandidaten; abgelehnt: { $rejected }.
       *[other] Das Modell schrieb { $attempts } Kandidaten; abgelehnt: { $rejected }.
    }
cli-rewrite-best-effort = Umschreiben geschieht nach bestem Bemühen: Es ändert den Wortlaut, und was es nicht feststellt, steht unten.
cli-rewrite-seed = Basis-Seed { $seed }; --seed { $seed } wiederholt diesen Lauf auf einem Modell dieses Rechners.
cli-prompts-unreadable = Die Vorlagendatei { $path } konnte nicht gelesen werden: { $reason }. Nichts wurde umgeschrieben.
cli-prompts-not-rows = Die Vorlagendatei { $path } ist kein JSON-Objekt aus Vorlagenzeilen ({ $reason }). Nichts wurde umgeschrieben.
cli-prompts-unknown-row = Die Vorlagendatei { $path } nennt { $key }, und diese Vorlagenzeile gibt es in dieser Version nicht. Nichts wurde umgeschrieben.
cli-prompts-invalid = Die Vorlage { $key } in { $path } bricht die Regel { $rule }. Nichts wurde umgeschrieben.

## The windows clean (E7)

toolbar-clean-all = Alles bereinigen
toolbar-clean-all-tooltip = Jede wartende Zeile bereinigen, die sich bereinigen lässt – eine nach der anderen, in der Reihenfolge ihres Eintreffens. Ausgegraut, solange es keine gibt.
queue-column-status = Status
queue-status-waiting = Wartet
queue-status-waiting-tooltip = Noch nicht bereinigt. Über das Menü „Aktionen“ bereinigen oder „Alles bereinigen“ drücken.
queue-status-unable = Nicht bereinigbar
queue-status-queued = Eingereiht
queue-status-queued-tooltip = Wartet auf die Bereinigung davor: Es wird immer eines nach dem anderen bereinigt, in der verlangten Reihenfolge.
queue-status-cleaning = Wird bereinigt…
queue-status-cleaning-tooltip = Wird gerade bereinigt. Geschrieben wird erst, wenn es fertig ist.
queue-status-nothing-found = Nichts gefunden
queue-status-cleaned = Bereinigt
queue-status-partly = Teilweise
queue-status-not-cleaned = Nicht bereinigt
queue-status-failed = Fehlgeschlagen
queue-action-clean = Bereinigen
queue-action-clean-done = Das wurde schon bereinigt.
queue-action-clean-busy = Das steht schon zur Bereinigung an.
queue-action-open-result = Ergebnis öffnen
queue-action-reveal-result = Ergebnis im Ordner zeigen
queue-action-copy-result = Ergebnis kopieren
queue-action-replace = Vorhandenes Ergebnis ersetzen
queue-went-written = Geschrieben als { $name }
queue-went-replaced = Über das vorhandene { $name } geschrieben
queue-went-in-place = Anstelle der Datei geschrieben
queue-went-set-aside = Original beiseitegelegt als { $name }
queue-went-kept = Aufbewahrt in { $folder }
queue-went-as-text = Der bereinigte Text liegt bereit: „Ergebnis kopieren“ steht im Menü „Aktionen“.
queue-went-nothing = Nichts wurde geschrieben.
status-cleaning = Bereinige { $current } von { $total }
clean-said-nothing-found = Es wurde nichts zum Entfernen gefunden, also wurde nichts geschrieben.
clean-said-cleaned-text = { $count ->
        [one] Ein Zeichen wurde entfernt oder ersetzt.
       *[other] { $count } Zeichen wurden entfernt oder ersetzt.
    }
clean-said-cleaned-picture = Was das Bild als von KI gemacht kennzeichnete, wurde entfernt.
clean-said-partly-kept = Gefunden wurde etwas, das bei den Standardeinstellungen bleibt – ein Buchstabe aus einem anderen Alphabet, der wie ein lateinischer aussieht –, also wurde nichts geändert.
clean-said-partly-mark = Ein sichtbares Zeichen ist noch im Bild: Es ließ sich nicht ganz entfernen.
clean-said-partly-animated = Die Einzelbilder eines animierten Bildes werden nicht auf ein sichtbares Zeichen geprüft; eines dort ist weder gefunden noch ausgeschlossen.
clean-said-partly-unexamined = Die Pixel des Bildes ließen sich nicht prüfen; ein sichtbares Zeichen dort ist weder gefunden noch ausgeschlossen.
clean-refused-not-yet = { $format }-Bilder werden in dieser Version noch nicht gelesen.
clean-refused-folder = Ein Ordner wird nicht als Ganzes bereinigt; legen Sie stattdessen die Dateien darin ab.
clean-refused-kind = Weder die Textbereinigung noch die Bildbereinigung liest so etwas: { $what }.
clean-refused-unnamed-encoding = Die Zeichen sind in einer Kodierung, die sich nicht benennen ließ, und eine Kodierung wird nie geraten.
clean-refused-unread = Aus dem Inhalt ließ sich nichts feststellen, und ein Name allein reicht zum Bereinigen nicht.
clean-refused-too-big = Mit { $size } ist das mehr, als ein Fenster bereinigt; die Grenze liegt bei { $limit }.
clean-refused-unreadable = Es ließ sich nicht lesen.
clean-refused-undecodable = An Byte { $offset } ist es kein gültiges { $encoding }, also wurde nichts geändert.
clean-refused-picture-unknown = Das ist kein Bild, das diese Version lesen kann.
clean-refused-picture-malformed = Das ist keine { $format }-Datei, die diese Version lesen kann: Sie ist an Byte { $offset } beschädigt. Nicht gelesen heißt nicht sauber.
clean-refused-picture-unsupported = Es verwendet an Byte { $offset } etwas in { $format }, das diese Version nicht unterstützt.
clean-refused-picture-decode = Die Pixel ließen sich nicht dekodieren, also wurde es nicht bereinigt.
clean-refused-picture-encode = Das wiederhergestellte Bild ließ sich nicht zurückschreiben, also wurde nichts geschrieben.
clean-refused-picture-proof = Das Ergebnis hat seine eigene Prüfung nicht bestanden, also wurde nichts geschrieben. Das ist ein Fehler dieser Version.
clean-refused-still-marked = Das Ergebnis trüge noch Metadaten zur KI-Herkunft, also wurde es nicht geschrieben.
clean-refused-exists = { $name } ist schon da und blieb, wie es war. Um es zu überschreiben, wählen Sie „Vorhandenes Ergebnis ersetzen“ im Menü „Aktionen“.
clean-refused-original-exists = { $name } ist schon da: Ein früher beiseitegelegtes Original wird nie überschrieben, also wurde nichts geändert.
clean-refused-link = { $name } ist ein symbolischer Link, und „Anstelle der Datei“ ersetzt eine Datei, keinen Link – also wurde nichts geändert. Bereinigen Sie die Datei, auf die er zeigt, oder wählen Sie auf der Seite „Aufbewahrung“ einen anderen Ort für Ergebnisse.
clean-refused-same-file = Das Ergebnis wäre auf der Datei selbst gelandet, also wurde nichts geschrieben.
clean-refused-nowhere = Nach den Einstellungen der Seite „Aufbewahrung“ kann dieses Ergebnis nirgendwohin.
clean-failed-write = { $path } ließ sich nicht schreiben ({ $error }). Sonst wurde nichts geändert.
clean-failed-set-aside = Die Datei ließ sich nicht als { $name } beiseitelegen ({ $error }), also wurde nichts geändert.
clean-failed-stranded = Das Ergebnis ließ sich nicht schreiben und das Original nicht zurücklegen: Es liegt unter { $path } ({ $error }).
clean-failed-panicked = Das Bereinigen brach an einem Fehler dieser Version ab, bevor es fertig war; der Fehler steht im Protokoll. Sehen Sie nach, wohin das Ergebnis gekommen wäre, bevor Sie es erneut bereinigen.
queue-action-report = Bericht…
window-report-title = Bericht · { $name }
window-report-arrived = Was ankam
window-report-happened = Was geschah
window-report-verifiable = Überprüfbar
window-report-best-effort = Nach bestem Bemühen
window-report-result = Das Ergebnis: { $path }
window-report-original = Das Original, beiseitegelegt: { $path }
window-report-kept = Aufbewahrte Kopien: { $path }
window-report-finding = { $codepoint } { $name } · { $class } · { $confidence } · { $count ->
        [one] einmal
       *[other] { $count }-mal
    }
window-report-removed-none = Die Bereinigung hat aus diesem Text nichts entfernt.
window-report-normalized = { $count ->
        [one] Ein Zeichen wurde normalisiert.
       *[other] { $count } Zeichen wurden normalisiert.
    }
window-report-kept-homoglyph = Bei den Standardeinstellungen behalten: Ein Buchstabe aus einer anderen Schrift wird nur von einer aggressiven Bereinigung ersetzt, und ein Fenster führt keine aus.
window-report-kept-in-place = Behalten, wo es eine Aufgabe hat: in einem Emoji oder einer Schrift, die es braucht.
window-report-picture-checked = Das Ergebnis wurde erneut gelesen: Es enthält keine Metadaten zur KI-Herkunft mehr.
window-report-picture-still = Erneut gelesen, trug das Ergebnis noch Metadaten zur KI-Herkunft.
window-report-shelf-empty = Für diese Bereinigung steht hier nichts.
window-report-not-read = Es wurde nicht gelesen, also wurde hier nichts geprüft.
window-report-copy-json = JSON kopieren
window-report-copy-markdown = Als Markdown kopieren
window-report-close = Schließen
window-report-copied = Kopiert.

panel-looking = Wird angesehen…
panel-found-text = { $count ->
        [one] Ein Zeichen zu entfernen oder zu ersetzen.
       *[other] { $count } Zeichen zu entfernen oder zu ersetzen.
    }
panel-found-text-nothing = Nichts zu entfernen.
panel-found-text-kept = Nichts zu entfernen; ein Buchstabe aus einem anderen Alphabet, der wie ein lateinischer aussieht, bleibt bei den Standardeinstellungen stehen.
panel-found-picture-both = KI-Metadaten und ein sichtbares Zeichen.
panel-found-picture-metadata = KI-Metadaten.
panel-found-picture-mark = Ein sichtbares Zeichen.
panel-found-picture-nothing = Nichts gefunden – weder in den Metadaten noch unter den sichtbaren Zeichen, die diese Version kennt.
panel-found-not-examined = { $metadata ->
        [yes] KI-Metadaten;
       *[no] Keine KI-Metadaten;
    } { $why ->
        [animated] Die Einzelbilder eines animierten Bildes werden nicht auf ein sichtbares Zeichen geprüft.
        [catalogue] Der Katalog sichtbarer Zeichen wurde nicht geladen, also wurden die Pixel nicht geprüft.
       *[decode] Die Pixel ließen sich nicht dekodieren, also wurden sie nicht geprüft.
    }
panel-clean = Bereinigen
panel-clean-tooltip = Bereinigen, was hier abgelegt wurde, eins nach dem anderen; jedes Ergebnis kommt dorthin, wo die Seite „Aufbewahrung“ es sagt. Ausgegraut, solange eine Bereinigung läuft oder hier nichts mehr zu bereinigen ist.
panel-cleaning = Wird bereinigt…

## E4-6b — the command line
cli-arg-record = Für diese Ansicht eine Zeile im Journal der Anwendung hinterlassen, das ihr Hauptfenster zeigt. Eine Ansicht ändert nichts und wird ohne dieses Flag nicht festgehalten.
cli-arg-no-record = Für diesen Lauf keine Zeile im Journal der Anwendung hinterlassen. Ohne das Flag erscheint der Lauf im Hauptfenster der Anwendung, als von der Befehlszeile gekommen.
cli-arg-out-rewrite = Ausgabedatei, oder `-` für die Standardausgabe. Standard ist `<name>.rewritten.<ext>` neben der Eingabe — das `<name>.cleaned.<ext>` einer Bereinigung ist ein anderes Ergebnis —, und die Standardausgabe, wenn die Eingabe die Standardeingabe ist; direktes Überschreiben braucht ein ausdrückliches Flag und ist nie der Standard.
cli-journal-too-old = Im Journal der Anwendung wurde nichts festgehalten: ihre Datenbank unter { $path } stammt von einer älteren Version, und die Anwendung bringt sie beim nächsten Start auf den neuesten Stand.
cli-journal-newer = Im Journal der Anwendung wurde nichts festgehalten: ihre Datenbank unter { $path } wurde von einer neueren Version von { -brand-name } geschrieben.
cli-journal-unwritable = Im Journal der Anwendung wurde nichts festgehalten: in ihre Datenbank unter { $path } konnte nicht geschrieben werden ({ $reason }).

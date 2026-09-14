### Wipemark — русский каталог.
###
### Набор ключей задаётся файлом `en-US/wipemark.ftl`; там же описано,
### что нужно соблюдать при переводе. Ключ, которого там нет, приводит
### к падению тестов.

-brand-name = Wipemark
-layer-a = очистка
-layer-b = переписывание

## Окно

window-title = { -brand-name }

## Три панели

toolbar-import = Импорт…
toolbar-import-tooltip = Выбрать файлы или папки. Они попадут сюда так же, как при перетаскивании.
toolbar-import-choose = Импортировать
toolbar-paste = Вставить
toolbar-paste-text = Вставить текст
toolbar-paste-image = Вставить картинку
toolbar-paste-files = { $count ->
        [one] Вставить файл
        [few] Вставить { $count } файла
       *[other] Вставить { $count } файлов
    }
toolbar-paste-items = { $count ->
        [one] Вставить { $count } элемент
        [few] Вставить { $count } элемента
       *[other] Вставить { $count } элементов
    }
toolbar-paste-tooltip = То, что лежит в буфере обмена, попадёт сюда так же, как при перетаскивании. Кнопка неактивна, пока в буфере нет ничего, что это окно может принять.
toolbar-help = Справка
toolbar-help-tooltip = Что делает это окно
toolbar-help-drop = Перетащите текст, картинку или файлы в любое место этого окна, нажмите «Импорт» и выберите их — или вставьте из буфера обмена.
toolbar-help-preview = Задержите указатель на превью, чтобы увидеть его крупнее. Меню действий в конце строки открывает файл той программой, которой открыла бы система.
toolbar-help-pending = Самой очистки в этой версии пока нет: окно принимает то, что попадает в него, и говорит, что это.
toolbar-help-elsewhere = Панель быстрой очистки — в строке меню; настройки — за шестерёнкой внизу справа.

queue-column-preview = Превью
queue-column-id = ID
queue-column-keyword = Ключевое слово
queue-column-name = Имя
queue-column-kind = Тип
queue-column-format = Формат
queue-column-size = Размер
queue-column-arrived = Получено
queue-column-actions = Действия
queue-empty-invite = Пока пусто. Перетащите текст, картинку или файлы, нажмите «Импорт» — или вставьте из буфера обмена.
queue-empty-release = Отпустите — и окно скажет, что это.
queue-count = { $count ->
        [one] { $count } элемент
        [few] { $count } элемента
       *[other] { $count } элементов
    }
queue-pending = Самой очистки в этой версии пока нет. Что есть уже сейчас: этот список принимает то, что перетащили или импортировали, и говорит, что это.
queue-preview-pending = Читаю…
queue-preview-cut = Первые { $count } символов; остальное здесь не показано.
queue-actions = Действия
queue-action-open = Открыть в программе по умолчанию
queue-action-compare = Сравнить с результатом
queue-copy = Скопировать
queue-filter-id = Поиск по ID
queue-filter-keyword = Поиск по ключевому слову
queue-reset-filters = Сбросить фильтры
queue-count-filtered = { $count } из { $total ->
        [one] { $total } элемента
        [few] { $total } элементов
       *[other] { $total } элементов
    }
queue-empty-filtered = Под эти фильтры ничего не подходит.
queue-sort-newest = Сначала новые
queue-sort-oldest = Сначала старые
queue-page-size = { $count } на странице
queue-page = Страница { $page } из { $pages }
queue-previous = Назад
queue-next = Вперёд

## Строка состояния

status-idle-no-engine = Простой · движок не настроен · только { -layer-a }
status-idle-here = Простой · { $model } · ничего не покидает этот компьютер · только { -layer-a }
status-idle-away = Простой · { $model } на { $host } · документ покинул бы этот компьютер · только { -layer-a }

## Настройки

settings-title = Настройки
settings-open = Настройки…
settings-appearance-title = Оформление
settings-appearance-description = Системная тема следует за рабочим столом, в том числе если он меняется при запущенном { -brand-name }.

settings-language-title = Язык
settings-language-description = Каждый язык подписан на нём самом. Системный язык следует за рабочим столом и определяется при запуске { -brand-name }.

settings-setup-title = Первичная настройка
settings-setup-description = Пошаговый проход, который открывается при первом запуске: на что хватает памяти этой машины, кто переписывает, и модель или адрес, которые для этого нужны. Он меняет те же строки, что страницы «Движок» и «Модели», и ничего больше.
settings-setup-run = Пройти снова…
settings-setup-reset = Забыть, что показывался (отладочная сборка)

## Первичная настройка — пошаговый проход.

setup-title = Настроить { -brand-name }
setup-skip = Пропустить
setup-back = Назад
setup-next = Далее
setup-finish = Готово

setup-step-welcome = Начало
setup-step-machine = Эта машина
setup-step-who = Кто переписывает
setup-step-model = Модель
setup-step-endpoint = Адрес
setup-step-done = Готово

setup-welcome-body = { -brand-name } убирает следы ИИ из вашего собственного содержимого в два слоя: { -layer-a } удаляет невидимые символы и детерминирована, а { -layer-b } просит языковую модель о перефразировании. В этой версии не работает ни то, ни другое. Эти шаги решают, что понадобится слою переписывания, когда он появится: кто переписывает и что для этого нужно.
setup-welcome-again = Всё, что здесь выбрано, потом можно изменить в настройках, а сам проход повторить со страницы «Основные».

setup-machine-reading = Читаю эту машину…
setup-machine-here = Машина сообщает о { $total } МБ памяти, и её хватает для { $model } — ей нужно около { $ram } МБ. Переписывание может остаться на этом компьютере, и это рекомендация.
setup-machine-tight = Машина сообщает о { $total } МБ памяти и вмещает { $model } — ей нужно около { $ram } МБ, — но почти ничего не остаётся на остальное. Оставить переписывание здесь по-прежнему рекомендуется; адрес — альтернатива.
setup-machine-away = Машина сообщает о { $total } МБ памяти, и ничего из каталога не помещается: самой маленькой модели, { $model }, не хватает { $short } МБ. Рекомендуется адрес — сервер где-то ещё, — и документ будет отправлен туда.
setup-machine-unjudged = Память этой машины прочитать не удалось, так что ничто с ней не сверяется и ничего не рекомендуется. На следующем шаге подойдёт любой выбор; { $model } — самая маленькая модель в каталоге.
setup-machine-nothing = В каталоге нет модели для переписывания, так что единственный способ его получить — адрес.
setup-machine-unified = Один пул памяти, общий для модели и всего остального, что запущено.
setup-machine-vram = Видеопамять: { $vram } МБ. Она решает, насколько быстро, а не будет ли модель работать вообще.
setup-machine-vram-unknown = Видеопамять: не измерена. Она решает, насколько быстро, а не будет ли модель работать, поэтому из-за неё ничто не отклоняется.

setup-who-body = Переписать документ могут двое, и различаются они в единственном важном: покидает ли документ этот компьютер.
setup-who-machine-line = Документ никогда не покидает этот компьютер. Нужна загруженная модель.
setup-who-endpoint-line = Сервер, который вы укажете. Документ отправляется ему, а удалённому ещё нужно «{ settings-engine-allow-remote-title }» на странице «Движок», прежде чем что-либо будет отправлено.
setup-who-ordered = Спросить одного, а затем другого — тоже вариант, на странице «Движок».

setup-model-body = Веса загружаются один раз, перед сохранением сверяются с контрольной суммой из каталога и остаются в папке данных { -brand-name }, пока их не удалят. Первая пришедшая модель выбирается для переписывания.
setup-model-others = Остальной каталог — на странице «Модели».
setup-open-models = Открыть страницу «Модели»…

setup-endpoint-body = Провайдер, адрес, имя модели и ключ — строки страницы «Движок», и этот шаг открывает её, а не повторяет их. Вернитесь сюда — строка ниже скажет, что из этих строк складывается.
setup-open-engine = Открыть страницу «Движок»…

setup-done-body = Всё выбранное здесь есть на страницах «Движок» и «Модели», а сам проход — в настройках, на странице «Основные».

settings-shortcut-panel-title = Сочетание клавиш для панели
settings-shortcut-panel-description = Вызывает панель поверх того, в чём вы сейчас работаете, и убирает её обратно. Здесь сочетание задано сразу: запишите другие клавиши, чтобы его сменить, Backspace стирает его насовсем, Escape оставляет прежнее.

settings-shortcut-show-title = Сочетание клавиш, которое показывает { -brand-name }
settings-shortcut-show-description = Работает из любого приложения, пока { -brand-name } запущен, — в том числе когда окно скрыто за значком в строке меню. Щёлкните по полю и нажмите клавиши; нужна клавиша-модификатор, кроме Shift. Backspace стирает сочетание, Escape оставляет прежнее.

hotkey-placeholder = Нажмите, чтобы задать
hotkey-recording = Нажмите клавиши…
hotkey-needs-modifier = Удерживайте ещё и клавишу-модификатор, кроме Shift.
hotkey-unrecordable = Эта клавиша не может быть частью сочетания.
hotkey-taken = Уже занято строкой «{ $action }».

hotkey-registered = Действует из любого приложения, пока { -brand-name } запущен.
hotkey-refused = Система не приняла его: { $reason }
hotkey-unavailable = Сохранено, но не действует: системных сочетаний клавиш на этой платформе пока нет.

## Разделы боковой панели

settings-section-general = Основные
settings-section-placement = Расположение
settings-section-compare = Сравнение
settings-section-engine = Движок
settings-section-mcp = MCP
settings-section-retention = Хранение

## Расположение окон

## Панель — окно, которое вызывают.

panel-title = Быстрая очистка
panel-pending = Самой очистки в этой версии пока нет. Сегодня настоящее здесь — то, что окно принимает брошенное и говорит, что это, и то, что оно открывается там, где вы указали.
panel-dismiss = Escape убирает её.

panel-help = Что тут можно сделать
panel-help-move = Тащите за любое место панели, чтобы подвинуть её.
panel-help-resize = Потяните за край или угол, чтобы изменить размер.
panel-help-dismiss = Escape убирает её; строка меню и её сочетание клавиш возвращают.
panel-help-placement = Где она открывается, запоминается для каждого экрана — в «Настройках», раздел «Расположение»; а если подвинуть её руками, это перебьёт выбранное там.

## Что попало на панель и чем оно оказалось.

panel-drop-invite = Перетащите сюда текст, картинку или файлы.
panel-drop-release = Отпустите — и станет видно, что это.
panel-drop-nothing = В этом перетаскивании не оказалось ничего, что можно прочитать.
panel-drop-more = … и ещё { $count ->
        [one] { $count } объект
        [few] { $count } объекта
       *[many] { $count } объектов
    }

panel-drop-by-name = Судим по имени — содержимое не говорит ни за, ни против.
panel-drop-mismatch = Имя обещает { $named }, а внутри — { $found }.

panel-drop-result-beside = Результат лёг бы рядом, как { $name }; сам файл остался бы нетронутым.
panel-drop-result-into-file = Результат лёг бы в { $folder }; сам файл остался бы нетронутым.
panel-drop-result-into = Результат лёг бы в { $folder }.
panel-drop-result-over = Результат занял бы его место, как только оригинал был бы отложен как { $name }.
panel-drop-result-as-text = Результат вернулся бы текстом; никакой файл не был бы записан.
panel-drop-each-file = С каждым файлом внутри поступили бы как с перетащенным файлом.
panel-drop-kept-originals = Копия оригинала хранилась бы { $period }.
panel-drop-kept-results = Копия результата хранилась бы { $period }.
panel-drop-kept-both = Копии оригинала и результата хранились бы { $period }.

kind-text = Текст
kind-image = Картинка
kind-document = Документ
kind-archive = Архив
kind-media = Звук или видео
kind-data = Данные
kind-folder = Папка
kind-unknown = Не опознано

## Окно сравнения — результат рядом со своим оригиналом.

compare-title = Сравнение · { $name }
compare-original = Оригинал
compare-result = Результат
compare-pending = Самой очистки в этой версии пока нет: результат начинается как копия оригинала. Отредактируйте его — и каждая строка, которая отличается, будет отмечена с обеих сторон.
compare-reading = Читаю…
compare-same = Результат — это оригинал, строка в строку.
compare-changed = { $added ->
        [one] { $added } строка добавлена
        [few] { $added } строки добавлены
       *[other] { $added } строк добавлено
    }, { $removed ->
        [one] { $removed } строка удалена
        [few] { $removed } строки удалены
       *[other] { $removed } строк удалено
    }
compare-refused-not-text = Это не текст, поэтому сравнивать строка за строкой нечего.
compare-refused-too-big = При размере { $size } это больше, чем сравнивает это окно; предел — { $limit }.
compare-refused-unreadable = Прочитать не удалось.
compare-reset = Вернуть оригинал
compare-reset-tooltip = Отбросить правки; результат снова становится оригиналом.
compare-help = Что делает это окно
compare-help-marks = Красная отметка у оригинала — строка, которой в результате больше нет; зелёная у результата — строка, которой в оригинале не было.
compare-help-follows = Оригинал следует за курсором в результате, чтобы обе стороны шли в ногу.
compare-help-toolbar = Панель над результатом — это собственные операции редактора с теми сочетаниями клавиш, на которые он и так отвечает.
compare-help-words = Внутри изменённого фрагмента слова, которые отличаются, выделены сильнее.
compare-help-characters = Внутри изменённого фрагмента символы, которые отличаются, выделены сильнее.
compare-help-settings = Что выделять и следует ли оригинал за курсором, выбирается на странице «Сравнение» в настройках — для следующего открытого окна.
compare-help-close = Закрытие этого окна ничего не записывает; результат живёт только здесь.

result-undo = Отменить
result-redo = Повторить
result-cut = Вырезать
result-copy = Скопировать
result-paste = Вставить
result-select-all = Выделить всё
result-indent = Увеличить отступ
result-outdent = Уменьшить отступ
result-find = Найти и заменить
result-soft-wrap = Переносить длинные строки
result-whitespace = Показывать пробелы


settings-compare-title = Как сравнивается результат
settings-compare-description = Что окно сравнения выделяет, когда результат стоит рядом с оригиналом. Окно читает эти настройки при открытии; уже открытое окно остаётся с теми, с которыми открылось.
settings-compare-exact = Важен каждый символ: сравнение не пропускает ни пробел, ни конец строки, ни символ, которого не видно.

settings-compare-grain-title = Что выделять
settings-compare-grain-description = Каждая строка, которая отличается, отмечается с обеих сторон при любом выборе здесь. Внутри изменённого фрагмента — а не просто добавленного или удалённого — выделение может быть точнее: слова, которые отличаются, или отдельные символы.
settings-compare-grain-lines = Только строки
settings-compare-grain-words = Изменённые слова
settings-compare-grain-characters = Изменённые символы

settings-compare-follow-title = Оригинал следует за курсором
settings-compare-follow-description = Когда курсор в результате перемещается, оригинал прокручивается к строке, которая стоит на том же месте, — так обе стороны идут в ногу. Если выключить, каждая сторона прокручивается сама по себе.

settings-placement-title = Где открываются окна
settings-placement-description = На каком экране открывается окно { -brand-name } и в какой части этого экрана.

settings-placement-looking = Читаем список экранов…
settings-placement-attached = { $count ->
        [one] Подключён { $count } экран.
        [few] Подключено { $count } экрана.
       *[other] Подключено { $count } экранов.
    }
settings-placement-only-window = Эти настройки расставляют панель { -brand-name } — окно, которое вызывают из строки меню. Окна рабочей области, когда появятся, будут открываться по тем же правилам; главное окно и это они не трогают.

settings-placement-close-title = Закрывать после перетаскивания
settings-placement-close-description = Когда окно бросают на часть экрана, это окно закрывается — чтобы было видно, куда оно встало. По клику по части экрана это окно остаётся открытым.

settings-placement-screen-title = Открывать на
settings-placement-screen-description = Активный экран — тот, на котором находится указатель в момент открытия окна. Основной — тот, на котором рабочий стол держит строку меню, где бы ни был указатель.
settings-placement-screen-active = Активный экран
settings-placement-screen-primary = Основной экран

settings-placement-display = Экран { $number }
settings-placement-resolution = { $width } × { $height }
settings-placement-primary = Основной
settings-placement-here = Панель здесь
settings-placement-opens-in = Окно открывается { $zone } на этом экране.
settings-placement-opens-where-left = Там, где вы его поставили на этом экране, и того размера, который задали.
settings-placement-where-it-was-left = Как я поставил
settings-placement-restore-default = Вернуть по умолчанию
settings-placement-drag-hint = Перетащите его в нужную часть экрана или нажмите на неё. Если подвинуть или растянуть саму панель, это перебьёт и то, и другое.

settings-placement-zone-top-left = сверху слева
settings-placement-zone-top-centre = сверху по центру
settings-placement-zone-top-right = сверху справа
settings-placement-zone-bottom-left = снизу слева
settings-placement-zone-bottom-centre = снизу по центру
settings-placement-zone-bottom-right = снизу справа

## Движок

settings-engine-title = Движок переписывания
settings-engine-description = Переписывание отправляет документ модели и оценивает то, что вернулось. Очистке движок не нужен никогда и за движком она не запирается.

settings-engine-pending = Эти настройки сохраняются, и пока никто ничего никуда не отправляет: переписывания в этой версии нет. До тех пор { -brand-name } только очищает, и в статусной строке так и написано.

settings-engine-state-off = Движка нет. { -brand-name } только очищает — само по себе детерминированно и самодостаточно.
settings-engine-state-ready-local = Настроено, и документ остался бы на этом компьютере: { $endpoint }
settings-engine-state-ready-machine = Настроено, и документ не покинул бы этот компьютер: здесь работает { $model }.
settings-engine-state-model-not-here = Модель, выбранная для переписывания, ещё не загружена на этот компьютер. Загрузите её на странице «Модели» или укажите здесь сервер.
settings-engine-state-model-unusable = Модель { $model }, выбранная для переписывания, не может быть использована этой версией для этой задачи. Выберите другую на странице «Модели».
settings-engine-state-no-such-profile = Профиля с именем «{ $name }» нет. Ничего не было подставлено вместо него.
settings-engine-state-ready-remote = Настроено. Документ был бы отправлен на { $endpoint } — это не этот компьютер.
settings-engine-state-no-model = Модель не названа. Каждый запрос должен говорить, какая модель на него отвечает.
settings-engine-state-no-model-chosen = На этом компьютере не выбрана модель для переписывания. Она выбирается на странице «Модели», когда загружена.
settings-engine-state-remote-refused = { $host } — не этот компьютер, а отправлять документы за его пределы не разрешено. Включите ниже «{ settings-engine-allow-remote-title }» или верните адрес на этот компьютер.
settings-engine-state-no-key = Для { $origin } ключ не сохранён. Этому поставщику он нужен.
settings-engine-state-key-in-the-clear = { $origin } — незашифрованное соединение с другим компьютером, поэтому ключ ушёл бы по сети открытым текстом. { -brand-name } его не отправит. Возьмите https или адрес на этом компьютере.
settings-engine-state-key-unreadable = Ключ не удалось прочитать: { $reason }
settings-engine-state-checking = Ищем сохранённый ключ…

settings-engine-serves-title = Кто переписывает
settings-engine-serves-description = Переписать документ могут двое: модель, загруженная на странице «Модели», — она никогда не покидает этот компьютер, — и адрес ниже, то есть сервер где-то. Порядок объявляется, а не скрывается: когда отвечает второй, уведомление выше говорит, кто именно и почему. Удалённому адресу по-прежнему нужен «{ settings-engine-allow-remote-title }», прежде чем что-либо будет отправлено.
settings-engine-serves-machine = Этот компьютер
settings-engine-serves-endpoint = Адрес
settings-engine-serves-machine-first = Компьютер, затем адрес
settings-engine-serves-endpoint-first = Адрес, затем компьютер
settings-engine-state-second-choice = Отвечает, потому что первый выбор не может: { $reason }

settings-engine-profile-title = Сохранённый профиль
settings-engine-profile-description = Все настройки этой страницы, кроме ключа, сохранённые под именем. Выбор профиля применяет их разом, а сохранение под уже занятым именем заменяет его. Ключ остаётся в хранилище учётных данных этого компьютера, привязанный к адресу, и общий для всех профилей, которые на него указывают.
settings-engine-profile-placeholder = Выберите сохранённый профиль
settings-engine-profile-name-placeholder = Назовите эти настройки
settings-engine-profile-save = Сохранить…
settings-engine-profile-delete = Удалить
settings-engine-profile-saved = Сохранено как «{ $name }».
settings-engine-profile-modified = «{ $name }», с несохранёнными изменениями.
settings-engine-profile-unsaved = Не сохранены под именем.
settings-engine-profile-no-key = Ключ в профиль не входит.

settings-engine-profile-name-title = Сохранить эти настройки
settings-engine-profile-name-body = Под новым именем или под тем, которое вы уже используете.
settings-engine-profile-name-taken = Заменить одно из этих:
settings-engine-profile-name-confirm = Сохранить
settings-engine-profile-delete-title = Удалить «{ $name }»?
settings-engine-profile-delete-body = Исчезнет только сохранённая копия. Настройки на этой странице останутся ровно такими же, и ключ в хранилище учётных данных этого компьютера тоже.
settings-engine-profile-delete-confirm = Удалить
settings-engine-profile-cancel = Отмена

settings-engine-provider-title = Поставщик
settings-engine-provider-description = Ollama говорит на своём /api/chat; второй достаёт до всего, что обслуживает /v1/chat/completions. Без движка { -layer-a } продолжает работать сам по себе.
settings-engine-provider-off = Без движка
settings-engine-provider-openai = Совместимый с OpenAI

settings-engine-endpoint-title = Адрес
settings-engine-endpoint-description = Базовый URL без пути — нужный { -brand-name } допишет сам. Только http и https, а URL с именем пользователя или паролем внутри отклоняется.

settings-engine-model-title = Модель
settings-engine-model-description = Имя, под которым модель известна этому адресу, ровно в его написании — llama3.1:8b, gpt-4o-mini, deepseek/deepseek-chat.

settings-engine-key-title = Ключ API
settings-engine-key-description = Хранится в хранилище учётных данных этого компьютера, под тем адресом, для которого был введён, и никогда не попадает в собственные настройки { -brand-name }. После сохранения он больше не показывается.
settings-engine-key-placeholder = Вставьте ключ, чтобы сохранить его
settings-engine-key-save = Сохранить
settings-engine-key-forget = Забыть
settings-engine-key-stored = Для { $origin } ключ сохранён.
settings-engine-key-absent = Для { $origin } ключ не сохранён.
settings-engine-key-not-used = Этот поставщик ключ не отправляет. У собственного API Ollama нет заголовка Authorization.
settings-engine-key-would-be-in-the-clear = { $origin } не зашифрован и это не этот компьютер. Сохранённый для него ключ мог бы уйти только открытым текстом, поэтому { -brand-name } его не отправит.
settings-engine-key-failed = Хранилище учётных данных отказало: { $reason }
settings-engine-key-not-persistent = На этом компьютере нет хранилища учётных данных, доступного { -brand-name }, поэтому введённый здесь ключ проживёт только до закрытия приложения.

settings-engine-allow-remote-title = Разрешить удалённый адрес
settings-engine-allow-remote-description = Выключено — адрес должен быть этим компьютером. Включено — текст каждого документа уходит тому, кто держит этот адрес; ради этого и берут размещённую модель, и это стоит выбрать, а не получить между делом.

settings-engine-temperature-title = Температура
settings-engine-temperature-description = От 0 до 2. Выше — дальше от исходных формулировок, ради чего переписывание и затевается и как заодно теряется факт.

settings-engine-reasoning-title = Усилие рассуждения
settings-engine-reasoning-description = В пересказе рассуждать не о чем. По умолчанию отправляется «none»; «off» убирает поле целиком — для серверов, которые это значение отвергают, а не пропускают мимо.
settings-engine-reasoning-off = Выключено (не слать)
settings-engine-reasoning-none = Никакого
settings-engine-reasoning-low = Малое
settings-engine-reasoning-medium = Среднее
settings-engine-reasoning-high = Высокое

settings-engine-timeout-title = Время ожидания
settings-engine-timeout-description = Сколько секунд ждать один ответ, прежде чем от него отказаться. Рассуждающая модель может потратить на одно предложение минуты.

## MCP

settings-section-models = Модели
settings-models-title = Локальные модели
settings-models-description = Открытые веса, загруженные на эту машину и сверенные с контрольной суммой из каталога { -brand-name }. Ничего не загружается, пока вы не попросите.
settings-models-pending = В этой версии загруженная модель — это файл на диске и ничего больше: в память её пока никто не загружает. Очистке она не нужна.
settings-models-folder-title = Папка моделей
settings-models-folder-description = Куда складываются загрузки и где { -brand-name } ищет файлы моделей — в этой папке и во всех вложенных. Пустое поле возвращает папку по умолчанию.
settings-models-folder-choose = Выбрать…
settings-models-folder-default = По умолчанию
settings-models-folder-busy = Дождитесь окончания загрузки, прежде чем переносить папку.
settings-models-folder-missing = Папка моделей: { $path } — её пока нет; первая загрузка её создаст.
settings-models-folder-unreadable = Папка моделей: { $path } — не удалось прочитать: { $reason }
settings-models-folder-read = Папка моделей: { $path } — { $installed ->
        [one] { $installed } модель из каталога
        [few] { $installed } модели из каталога
       *[other] { $installed } моделей из каталога
    }, { $other ->
        [0] больше ничего похожего на модель
        [one] ещё { $other } файл модели
        [few] ещё { $other } файла моделей
       *[other] ещё { $other } файлов моделей
    }.
settings-models-found-title = Ещё в этой папке
settings-models-found-description = Файлы моделей, найденные при обходе папки и всех вложенных. Их нет в каталоге этой версии, поэтому здесь нечем их проверить, и в работу их пока ничто не берёт.
settings-models-rewrite-title = Модель для переписывания
settings-models-rewrite-description = Какую загруженную модель использовал бы { -layer-b }. Перечислены только модели, уже находящиеся на этой машине; модель выбирается под назначение, а переписывание — единственное назначение, для которого эта сборка предлагает веса.
settings-models-rewrite-none = Без локальной модели
settings-models-size = Загрузка { $size }
settings-models-needs = Нужно около { $ram } МБ
settings-models-download = Загрузить
settings-models-resume = Продолжить
settings-models-cancel = Остановить
settings-models-remove = Удалить
settings-models-installed = На этой машине
settings-models-progress = { $done } из { $total }
settings-models-verifying = Проверяем, что уже есть…
settings-models-host = Машина сообщает о { $ram } МБ памяти.
settings-models-host-unknown = Объём памяти машины прочитать не удалось, поэтому ничего ниже с ним не сверяется.
settings-models-fit-roomy = Место есть, и остальной машиной по-прежнему можно пользоваться.
settings-models-recommended = Рекомендуется для этого компьютера
settings-models-fit-tight = Поместится, но почти ничего не останется.
settings-models-fit-too-big = На { $short } МБ памяти больше, чем есть на этой машине.
settings-models-fit-unknown = Не оценено: объём памяти машины прочитать не удалось.
settings-models-damaged = На машине есть, но это не то, что описано в каталоге. Удалите и загрузите заново.
settings-models-failed = Загрузка прервана: { $reason }
settings-models-stopped = Остановлено. Загруженное сохранено, следующая попытка продолжит с этого места.

settings-retention-title = Что сохраняется
settings-retention-description = Куда ложится результат, что происходит с файлом, из которого он получен, и хранит ли { -brand-name } копию того, что пришло без файла.

settings-retention-beside = Результаты записываются рядом с файлом, как name.cleaned.ext; сам файл никогда не трогается.
settings-retention-into = Результаты записываются в { $folder }; сам файл никогда не трогается.
settings-retention-over = Файл заменяется своим результатом, как только оригинал отложен как name.original.ext — и уже лежащий там оригинал никогда не перезаписывается.
settings-retention-keeps-nothing = То, что пришло без файла — вставленное, перетащенное из браузера — не хранится, как только результат его заменил.
settings-retention-keeps-originals = Оригинал вставленного или перетащенного хранится в { $folder } { $period }; результаты — нет.
settings-retention-keeps-results = Результат вставленного или перетащенного хранится в { $folder } { $period }; оригиналы — нет.
settings-retention-keeps-both = Оригинал и результат вставленного или перетащенного хранятся в { $folder } { $period }.
settings-retention-pending = Пока ничего не записывается: { -layer-a } и { -layer-b } в этой версии отсутствуют. Эти настройки решают, что случится с файлом и со вставленным, когда они появятся.

settings-retention-destination-title = Куда ложатся результаты
settings-retention-destination-description = «Рядом с файлом» записывает name.cleaned.ext по соседству и оставляет файл как есть. Папка результатов — та, что ниже. «Вместо файла» заменяет его — после того как оригинал отложен как name.original.ext, и никогда поверх уже лежащего там оригинала.
settings-retention-destination-beside = Рядом с файлом
settings-retention-destination-folder = В папку результатов
settings-retention-destination-replace = Вместо файла

settings-retention-folder-title = Папка результатов
settings-retention-folder-description = Куда ложатся результаты, когда они складываются в одну папку, — и куда ложится результат, которому не рядом с чем лечь, например картинка, перетащенная из браузера. Пустое поле возвращает папку «Загрузки».

settings-retention-originals-title = Хранить вставленное
settings-retention-originals-description = За вставленным текстом и перетащенными картинками нет файла, и как только результат их заменил, оригинала больше нет. Хранит копию в собственной папке { -brand-name } на срок ниже — как пришло, с разметкой. Файл сюда никогда не копируется: файл и есть оригинал.

settings-retention-results-title = Хранить результаты
settings-retention-results-description = Результат вставленного или перетащенного хранится в собственной папке { -brand-name } на срок ниже, чтобы до него можно было дотянуться, когда буфер обмена уже занят другим. Результат, записанный в файл, сюда не копируется.

settings-retention-period-title = Как долго
settings-retention-period-description = Сколько хранится копия, прежде чем { -brand-name } её удалит. За пределами собственной папки { -brand-name } это ничего не удаляет.
settings-retention-period-day = День
settings-retention-period-week = Неделю
settings-retention-period-month = Месяц
settings-retention-period-quarter = Три месяца
settings-retention-period-forever = Пока не удалите вручную
settings-retention-span-day = один день
settings-retention-span-week = одну неделю
settings-retention-span-month = один месяц
settings-retention-span-quarter = три месяца
settings-retention-span-forever = пока не удалите вручную

settings-mcp-title = Сервер MCP
settings-mcp-description = Агент сможет применять { -layer-a } к собственному тексту по протоколу Model Context Protocol.
settings-mcp-tools-pending = Сервер отвечает, а его инструменты — ещё нет: очистки в этой версии нет, и до тех пор каждый вызов отклоняется, а не возвращает отчёт ни о чём.

settings-mcp-status-off = Не запущен.
settings-mcp-status-starting = Запускается…
settings-mcp-status-listening = Работает и отвечает на { $url }
settings-mcp-status-moved = Порт { $wanted } был занят, поэтому взят { $port }. Ниже — тот сниппет, который сработает.
settings-mcp-status-failed = Не удалось запустить: { $reason }

settings-mcp-exposed = { $address } доступен из сети, и этот сервер не спрашивает пароль. Всё, что дотянется до этого компьютера, сможет запустить на нём { -layer-a }.

settings-mcp-enabled-title = Отдавать по MCP
settings-mcp-enabled-description = Запускается вместе с { -brand-name } и работает, пока работает он. Правка адреса или порта ниже перезапускает его.

settings-mcp-bind-title = Слушать на
settings-mcp-bind-description = Любой адрес этого компьютера — 127.0.0.1 отвечает только ему самому, 0.0.0.0 отвечает всем, кто дотянется, а 192.168.1.101 отвечает только на этом интерфейсе.

settings-mcp-port-title = Порт
settings-mcp-port-description = От 1024 до 65535. Занятый порт пропускается: сервер берёт следующий свободный и сообщает, какой.

settings-mcp-endpoint = Клиенты подключаются к { $url }

settings-mcp-snippets-title = Подключить клиента
settings-mcp-snippets-description = Вставьте это в конфигурацию клиента. Добавьте в существующий блок mcpServers, а не замените его.
settings-mcp-copy = Копировать
settings-mcp-copied = Скопировано

settings-mcp-client-generic = Любой клиент MCP

## Оформление

theme-system = Системная
theme-light = Светлая
theme-dark = Тёмная

## Язык

language-autonym = Русский
language-selector-label = Язык
language-system = Системный ({ $language })

## Третья полка (спецификация §0.1, правило 3)

report-not-established-title = Не установлено
report-not-established-vendor-detector-evasion = обход собственного детектора поставщика — не проверялось, оракула для этого не существует
report-not-established-human-authorship = авторство человека — ни одна проверка этого инструмента его не устанавливает
report-not-established-unknown-mark-schemes = метки в схемах, которые эта сборка не реализует, — не искались

## Строка меню

tray-show = Показать { -brand-name }
tray-panel = Показать панель
tray-clean-clipboard = Очистить буфер обмена — пока нет
tray-appearance = Оформление
tray-quit = Выйти из { -brand-name }

## Командная строка

cli-about = Удаляет метки происхождения ИИ из ваших собственных текстов и изображений

cli-help-usage = Использование:
cli-help-commands = Команды:
cli-help-arguments = Аргументы:
cli-help-options = Параметры:
cli-help-print-help = Показать справку
cli-help-print-version = Показать версию
cli-command-help = Показать эту справку или справку по указанной команде.

cli-command-inspect = Показывает, что находится в документе, не изменяя его.
cli-command-clean = Только { -layer-a }: детерминированно, проверяемо, без модели.
cli-command-rewrite = { -layer-a }, затем перезапись моделью, затем снова { -layer-a }.
cli-command-models = Управление загруженными весами.
cli-command-models-list = Показать манифест и то, что установлено.
cli-command-models-pull = Загрузить модель по идентификатору, докачивая начатый файл.
cli-command-models-verify = Пересчитать хеш установленной модели и сверить с манифестом.
cli-command-models-rm = Удалить установленную модель.
cli-command-audit = Обойти каталог и сообщить о находках, для CI.

cli-arg-path-or-stdin = Файл для чтения или `-` для стандартного ввода.
cli-arg-path = Файл для чтения.
cli-arg-out = Файл вывода. По умолчанию `<name>.cleaned.<ext>` рядом с исходным; запись поверх исходника требует явного флага и никогда не делается по умолчанию.
cli-arg-nfkc = Применить нормализацию NFKC (по умолчанию выключена — она меняет больше, чем метки происхождения).
cli-arg-aggressive = Обрабатывать также омоглифы и нетипичные пробелы. Выше доля ложных срабатываний, поэтому только по требованию.
cli-arg-json = Машиночитаемый JSON вместо обычного текста.
cli-arg-engine = Движок перезаписи: `local` или `remote`.
cli-arg-model = Идентификатор модели из манифеста.
cli-arg-tactic = Ступень лестницы тактик: paraphrase, humanize, back_translate, structural, code.
cli-arg-candidates = Число кандидатов на фрагмент.
cli-arg-rounds = Число проходов на фрагмент.
cli-arg-force = Продолжить, даже если движок перезаписи принадлежит тому поставщику, который предположительно и пометил документ, — метка тогда, скорее всего, будет проставлена снова.
cli-arg-id = Идентификатор модели из манифеста.
cli-arg-dir = Каталог для обхода.
cli-arg-sarif = Вывод в формате SARIF, для панелей сканирования кода.
cli-arg-language = Язык сообщений и справки, тег BCP-47, например de или ru. Имеет приоритет над WIPEMARK_LANG, языком, сохранённым в настройках приложения, и операционной системой — именно в этом порядке.

cli-unknown-language = неизвестный язык `{ $requested }`, выполняется откат. Доступны: { $available }

cli-not-implemented =
    разобрано `{ $summary }`, но `{ $command }` пока не реализовано.
    В этой версии набор аргументов и коды возврата окончательны,
    поведение — нет. Выход с кодом 2, а не 0: хук, который проходит
    потому, что ничего не выполнялось, хуже, чем отсутствие хука.

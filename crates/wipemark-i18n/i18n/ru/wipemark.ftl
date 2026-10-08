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
toolbar-help-pending = «Очистить» и «Переписать» есть в каждой строке и в её меню «Действия»; «Очистить всё» и «Переписать всё» берут каждую не начатую строку. Очистка идёт сразу, слоем { -layer-a }; переписывание ждёт своей очереди в единой очереди переписываний — этого окна, агента и командной строки — на дежурном движке, а «Переписать всё» сначала называет цену. Результаты ложатся туда, куда велит страница «Хранение».
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
queue-pending = Этот список очищает и переписывает — по одному переписыванию за раз, дежурным движком, кто бы ни просил: это окно, агент или командная строка, и строка говорит, кто. Завершённые строки остаются, пока их не уберут или не истечёт срок со страницы «Хранение».
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
status-idle-here = Простой · { $model } · ничего не покидает этот компьютер
status-idle-away = Простой · { $model } на { $host } · документ покинул бы этот компьютер
status-local-loading = Загружается { $model } · ничего не покидает этот компьютер
status-local-loading-progress = Загружается { $model } — { $percent } % · ничего не покидает этот компьютер
status-local-loaded = { $model } загружена · { -brand-name } занимает { $ram } · ничего не покидает этот компьютер
status-local-loaded-unmeasured = { $model } загружена · ничего не покидает этот компьютер
status-local-failed = { $model } не удалось загрузить: { $reason } · только { -layer-a }

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

setup-welcome-body = { -brand-name } убирает следы ИИ из вашего собственного содержимого в два слоя: { -layer-a } удаляет невидимые символы и детерминирована, а { -layer-b } просит языковую модель о перефразировании. Оба работают из этих окон — «Очистить» и «Переписать» в главном окне, «Очистить» на панели, — а также из командной строки и для агента по MCP. Эти шаги решают, что нужно слою переписывания: кто переписывает и что для этого нужно.
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
panel-pending = Это окно очищает; переписывание — в главном окне: «Переписать» в строке или «Переписать всё».
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
compare-pending = В окне сравнения результат — это то, что очистка делает из оригинала, и каждая строка, которая отличается, отмечена с обеих сторон. Правки результата там ничего не сохраняют, а закрытие окна ничего не записывает.
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
compare-reset = Вернуть очищенный текст
compare-reset-tooltip = Отбросить правки; результат снова становится тем, что очистка сделала из оригинала.
compare-help = Что делает это окно
compare-help-marks = Красная отметка у оригинала — строка, которой в результате больше нет; зелёная у результата — строка, которой в оригинале не было.
compare-help-follows = Оригинал следует за курсором в результате, чтобы обе стороны шли в ногу.
compare-help-scrolls = Прокрутка одной стороны прокручивает другую, так что общие строки стоят на одном уровне.
compare-help-toolbar = Панель над результатом — это собственные операции редактора с теми сочетаниями клавиш, на которые он и так отвечает.
compare-help-words = Внутри изменённого фрагмента слова, которые отличаются, выделены сильнее.
compare-help-characters = Внутри изменённого фрагмента символы, которые отличаются, выделены сильнее.
compare-help-settings = На странице «Сравнение» в настройках выбираются три вещи для следующего открытого окна: что выделять, следует ли оригинал за курсором и прокручиваются ли стороны вместе.
compare-help-close = Закрытие этого окна ничего не записывает; правки результата живут только здесь.

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
settings-compare-follow-description = Когда курсор в результате перемещается, курсор оригинала встаёт на строку, которая стоит на том же месте. Если выключить, курсор оригинала остаётся где был.

settings-compare-sync-scroll-title = Стороны прокручиваются вместе
settings-compare-sync-scroll-description = Прокрутка любой стороны — колесом, тачпадом, полосой прокрутки или клавиатурой — прокручивает и другую, так что общие строки двух сторон стоят на одном уровне. Внутри изменённого фрагмента другая сторона идёт в ногу по своим строкам. Пока результат переносит строки, стороны выравниваются лишь приблизительно. Если выключить, каждая сторона прокручивается сама по себе.

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

settings-engine-pending = «Переписать» в главном окне, переписывание агента через сервер MCP и wipemark-cli rewrite, пока это приложение запущено, — всё уходит тому, кого эта страница назначила: сюда или на сервер, по одному за раз. Единственный запрос самой страницы — проверка ниже, и она отправляет заранее заданную фразу.

settings-engine-state-off = Движка нет. { -brand-name } только очищает — само по себе детерминированно и самодостаточно.
settings-engine-state-ready-local = Настроено, и документ остался бы на этом компьютере: { $endpoint }
settings-engine-state-ready-machine = Настроено, и документ не покинул бы этот компьютер: здесь работает { $model }.
settings-engine-state-model-not-here = Модель, выбранная для переписывания, ещё не загружена на этот компьютер. Загрузите её на странице «Модели» или укажите здесь сервер.
settings-engine-state-model-unusable = Модель { $model }, выбранная для переписывания, не может быть использована этой версией для этой задачи. Выберите другую на странице «Модели».
settings-engine-state-added-model-changed = Добавленная вами модель, выбранная для переписывания, изменилась после добавления, поэтому она не загружается. Добавьте её заново на странице «Модели» или выберите другую.
settings-engine-state-added-model-not-here = Добавленной вами модели, выбранной для переписывания, больше нет на прежнем месте. Верните файл или выберите другую на странице «Модели».
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

## Модель на этом компьютере.

settings-engine-keep-title = Держать модель загруженной
settings-engine-keep-description = Для модели на этом компьютере; у сервера здесь держать нечего. «{ settings-engine-keep-on-demand }» загружает модель, когда она нужна, и освобождает её память через указанные ниже минуты без работы. «{ settings-engine-keep-resident }» загружает её вскоре после запуска { -brand-name } и держит, пока { -brand-name } не закроется или не будет выбрана другая модель. «{ settings-engine-local-unload }» освобождает её в любом случае.
settings-engine-keep-on-demand = Загружать при необходимости
settings-engine-keep-resident = Держать загруженной
settings-engine-idle-title = Выгружать через
settings-engine-idle-description = Сколько модель, загруженная при необходимости, остаётся в памяти без работы. Не действует, пока модель держится загруженной.
settings-engine-idle-minutes = { $count ->
        [one] { $count } минуту
        [few] { $count } минуты
       *[other] { $count } минут
    }
settings-engine-advanced = Дополнительно
settings-engine-lock-title = Держать модель в оперативной памяти (не давать системе выгружать её на диск)
settings-engine-lock-description = Тогда первый запрос после паузы не замедляется чтением модели обратно с диска; цена — эта память, которую ни одна другая программа не сможет занять, пока модель загружена. Если система откажет в блокировке, модель всё равно загрузится, а в журнале об этом будет запись.

settings-engine-local-title = Модель на этом компьютере
settings-engine-local-not-here = То, что сейчас отвечает за переписывание, работает не на этом компьютере, поэтому загружать здесь нечего.
settings-engine-local-not-loaded = { $model } не загружена.
settings-engine-local-resident-again = По-прежнему выбрано «{ settings-engine-keep-resident }», поэтому при следующем запуске { -brand-name } она загрузится снова.
settings-engine-local-loading = Загружается { $model }…
settings-engine-local-loading-progress = Загружается { $model } — прочитано { $percent } %…
settings-engine-local-loaded = { $model } загружена. { -brand-name } занимает { $ram } памяти, измерено. Загружена в { $since }.
settings-engine-local-loaded-unmeasured = { $model } загружена, с { $since }. Сколько памяти она занимает, прочитать не удалось.
settings-engine-local-file = Файл модели занимает { $size } на диске.
settings-engine-local-failed = Не удалось загрузить: { $reason }
settings-engine-local-unload = Выгрузить сейчас
settings-engine-local-unload-tooltip = Освободить память модели сейчас. Следующий запрос загрузит её снова.
settings-engine-local-unload-disabled = Модель не загружена, выгружать нечего.
settings-engine-local-check = Проверить
settings-engine-local-check-tooltip = Загрузить модель, если она не загружена, и попросить её написать несколько слов.
settings-engine-local-check-cancel = Отменить
settings-engine-local-checking = Проверка…
settings-engine-local-check-answered = Модель ответила: «{ $text }»
settings-engine-local-check-load = Загрузка заняла { $seconds } с.
settings-engine-local-check-speed = Токенов: { $tokens }, { $rate } в секунду после первого.
settings-engine-local-check-speed-unknown = Токенов: { $tokens } — слишком мало, чтобы измерить скорость.
settings-engine-local-check-failed = Проверка не выполнена: { $reason }
settings-engine-local-check-cancelled = Проверка отменена.
settings-engine-local-check-note = Проверка показывает, что модель загружается и пишет. Это не переписывание: эти окна пока ничего не переписывают, а агент по MCP и командная строка переписывают этой моделью.
settings-engine-local-no-tray = На этой системе нет значка в строке меню, поэтому закрытие главного окна завершает { -brand-name } и освобождает модель.

settings-engine-remote-title = Сервер
settings-engine-remote-asks = Проверка обращается к { $model } по адресу { $endpoint }.
settings-engine-remote-refused = К серверу нельзя обратиться: { $reason }
settings-engine-remote-check-tooltip = Отправить серверу заранее заданную фразу и показать, что он ответит.
settings-engine-remote-check-answered = Сервер ответил: «{ $text }»
settings-engine-remote-check-first = Первый фрагмент пришёл через { $seconds } с.
settings-engine-remote-check-speed = Фрагментов: { $pieces }, { $rate } в секунду после первого.
settings-engine-remote-check-speed-unknown = Фрагментов: { $pieces } — слишком мало, чтобы измерить скорость.
settings-engine-remote-check-note = Проверка показывает, что сервер отвечает. Это не переписывание: эти окна пока ничего не переписывают, а агент по MCP и командная строка отправляют документы сюда.
settings-engine-remote-check-sent-to = Её запрос — заранее заданная фраза, никогда не документ — отправляется на { $origin }, а это не этот компьютер.

engine-refusal-not-built = В этой сборке нет локального движка.
engine-refusal-no-such-file = Файла модели нет на месте: { $path }
engine-refusal-would-not-fit = Модели нужно около { $need }, а у этого компьютера { $have }.
engine-refusal-no-backend = Не найден процессор, на котором можно запустить модель.
engine-refusal-load-failed = Модель не удалось загрузить.
engine-refusal-stopped = Локальный движок остановился. Повторный выбор модели перезапускает его.
engine-refusal-nothing-on-duty = Отвечать некому: ничто не назначено.
engine-refusal-redirected = Сервер ответил { $status } и указал на { $origin }. Перенаправлениям { -brand-name } не следует: исправьте адрес.
engine-refusal-redirected-nowhere = Сервер ответил { $status } — это перенаправление. Перенаправлениям { -brand-name } не следует: исправьте адрес.
engine-refusal-key-rejected = Сервер не принял ключ ({ $status }).
engine-refusal-not-found = У сервера нет такой модели или такого адреса. Проверьте имя модели — для Ollama ещё и то, что она скачана командой pull.
engine-refusal-rate-limited-for = Сервер ограничивает запросы и просит подождать { $seconds } с.
engine-refusal-rate-limited = Сервер ограничивает запросы. Попробуйте позже.
engine-refusal-refused = Сервер отклонил запрос ({ $status }).
engine-refusal-key-unreadable = Ключ не удалось прочитать из хранилища учётных данных: { $reason }
engine-refusal-no-key = Для этого сервера ключ не сохранён.
engine-refusal-key-unsendable-empty = Ключ, сохранённый для этого сервера, пуст, и ничего не отправлено. Сохраните ключ заново на странице «Движок».
engine-refusal-key-unsendable-not-ascii = В ключе, сохранённом для этого сервера, есть символ вне ASCII, который запрос не может передать, и ничего не отправлено. Сохраните ключ заново на странице «Движок».
engine-refusal-key-unsendable-control = В ключе, сохранённом для этого сервера, есть управляющий символ, который запрос не может передать, и ничего не отправлено. Сохраните ключ заново на странице «Движок».
engine-refusal-key-unsendable-space = Внутри ключа, сохранённого для этого сервера, есть пробел, который запрос не может передать, и ничего не отправлено. Сохраните ключ заново на странице «Движок».
engine-refusal-chat-format-no-template = В этой модели нет шаблона чата, поэтому { -brand-name } не может составить для неё разговор. Выберите другую модель.
engine-refusal-chat-format-unrecognised = Формат чата этой модели пока не поддерживается: { -brand-name } не распознаёт её шаблон чата и не подбирает его наугад. Выберите другую модель.

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
settings-engine-key-refused-not-ascii = Не сохранено: в ключе есть символ вне ASCII — буква другой раскладки, типографское тире или невидимый символ, попавший при вставке, — и запрос не может его передать. Вставьте ключ ещё раз.
settings-engine-key-refused-control = Не сохранено: в ключе есть управляющий символ, который запрос не может передать. Вставьте ключ ещё раз.
settings-engine-key-refused-space = Не сохранено: внутри ключа есть пробел, а ни один провайдер не выдаёт ключей с пробелом. Вставьте ключ ещё раз.
settings-engine-key-refused-empty = Не сохранено: в поле нет ключа.
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
settings-models-pending = Загруженную модель можно загрузить в память и проверить на странице «Движок»; эти окна пока ею не переписывают, а агент по MCP и wipemark-cli rewrite — могут. Очистке она не нужна.
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
settings-models-found-description = Файлы моделей, найденные при обходе папки и всех вложенных, которых нет в каталоге { -brand-name }. Добавьте файл как модель, чтобы им пользоваться: { -brand-name } тогда запишет его контрольную сумму, но не может поручиться за то, что это за модель.
settings-models-rewrite-title = Модель для переписывания
settings-models-rewrite-description = Какую модель этого компьютера использовал бы { -layer-b } — скачанную из каталога или добавленную вами. Перечислены только модели, уже находящиеся на этой машине; модель выбирается под назначение, а переписывание — единственное назначение, для которого эта сборка предлагает веса.
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
settings-models-checking = Сверяем с каталогом: { $done } из { $total }…
settings-models-host = Машина сообщает о { $ram } МБ памяти.
settings-models-host-unknown = Объём памяти машины прочитать не удалось, поэтому ничего ниже с ним не сверяется.
settings-models-fit-roomy = Место есть, и остальной машиной по-прежнему можно пользоваться.
settings-models-recommended = Рекомендуется для этого компьютера
settings-models-fit-tight = Поместится, но почти ничего не останется.
settings-models-fit-too-big = На { $short } МБ памяти больше, чем есть на этой машине.
settings-models-fit-unknown = Не оценено: объём памяти машины прочитать не удалось.
settings-models-damaged = На машине есть, но это не то, что описано в каталоге. Удалите и загрузите заново.
settings-models-found-at = Найдена здесь: { $path }. { -brand-name } не загружал этот файл, поэтому использует его на месте и никогда не удаляет.
settings-models-foreign = Файл { $path } называется как эта модель, но это не она. { -brand-name } его не загружал, поэтому не использует и не удаляет; уберите его, чтобы загрузить эту модель сюда.
settings-models-foreign-part = О частичном файле { $path } у { -brand-name } нет записи: это загрузка другой программы или файл, запись о котором с ним больше не совпадает. Поэтому он не докачивается и не удаляется; уберите его, чтобы загрузить эту модель сюда.
settings-models-loading = Загружается в память — прочитано { $percent } %…
settings-models-failed = Загрузка прервана: { $reason }
settings-models-stopped = Остановлено. Загруженное сохранено, следующая попытка продолжит с этого места.
settings-models-add-file = Добавить файл модели…
settings-models-add-file-description = Файл GGUF в любом месте этого компьютера, которого нет в каталоге. { -brand-name } запоминает его контрольную сумму при добавлении и не может поручиться за то, что это за модель.
settings-models-add-as = Добавить как модель…
settings-models-added-by-you = Добавлена вами
settings-models-user-needs = Нужно около { $ram } МБ (оценка)
settings-models-user-context = Контекст { $context } токенов
settings-models-user-changed = Изменилась после добавления: это не тот файл, чья контрольная сумма записана, поэтому модель не загружается. Добавьте её заново, чтобы использовать в нынешнем виде, или забудьте её.
settings-models-user-missing = Файла больше нет на месте, поэтому модель не загружается. Верните его и проверьте заново или забудьте модель.
settings-models-user-unreadable = Файл не удалось прочитать: { $reason }
settings-models-user-chat-refused = Её формат чата пока не поддерживается, поэтому загрузка будет отклонена.
settings-models-user-adding = Добавление — прочитано { $done } из { $total }, чтобы записать контрольную сумму…
settings-models-user-add-failed = { $name } не добавлена: { $reason }
settings-models-forget = Забыть
settings-models-recheck = Проверить заново
settings-models-add-again = Добавить заново…
settings-models-forget-title = Забыть { $name }?
settings-models-forget-body = Файл остаётся на месте: { -brand-name } никогда не удаляет файл, который не скачивал. Модель перестаёт показываться и использоваться, а повторное добавление прочитает её целиком.
settings-models-forget-confirm = Забыть
settings-models-add-title = Добавить модель
settings-models-add-not-catalogue = Этой модели нет в каталоге { -brand-name }. { -brand-name } сейчас запоминает контрольную сумму файла и потом отклонит файл, если он изменится, но не может поручиться за то, что это за модель.
settings-models-add-file-line = { $path } · { $size }
settings-models-add-architecture = Архитектура: { $arch }
settings-models-add-weights = Параметры: { $params } · квантизация: { $quant }
settings-models-add-not-stated = не указано
settings-models-add-trained = Обучена с контекстом { $tokens } токенов.
settings-models-add-trained-unknown = Заголовок не сообщает, с каким контекстом модель обучена.
settings-models-add-chat-supported = Формат чата: поддерживается ({ $family }).
settings-models-add-chat-no-template = Формат чата: не поддерживается — в файле нет шаблона чата. Загрузка будет отклонена.
settings-models-add-chat-unrecognised = Формат чата: пока не поддерживается — { -brand-name } не распознаёт её шаблон чата и не подбирает его наугад. Загрузка будет отклонена.
settings-models-add-chat-not-built = Формат чата: не оценивается — эта сборка не может запускать модели на этом компьютере.
settings-models-add-memory = Память: около { $total } при этом контексте — оценка по заголовку файла: { $weights } на веса, { $cache } на кэш контекста и { $overhead } на работу. Сколько займёт загрузка, измеряется после неё.
settings-models-add-memory-coarse = Заголовок не описывает кэш контекста, поэтому взят типичный.
settings-models-add-name = Название
settings-models-add-purpose = Назначение
settings-models-add-context = Контекст, в токенах
settings-models-add-context-bounds = От { $min } до { $max } токенов.
settings-models-add-again-note = Этот файл уже добавлен как { $name }. Повторное добавление прочитает его целиком и запишет контрольную сумму файла в нынешнем виде.
settings-models-add-confirm = Добавить
settings-models-add-cancel = Отмена
settings-models-add-reading = Читается заголовок файла…
settings-models-add-unreadable = { $path } не удалось добавить: его заголовок не прочитан ({ $reason }).
settings-models-add-not-offered = { $path } не предлагается как модель. { $why }
settings-models-role-rewrite = Переписывание
models-not-offered-projector = Проектор изображения или звука (mmproj): половина мультимодальной модели, а не модель сама по себе.
models-not-offered-adapter = Адаптер LoRA: он меняет модель, но сам моделью не является.
models-not-offered-no-weights = В нём словарь и нет весов.
models-not-offered-not-a-writer = Модель эмбеддингов, энкодер или речевая модель, а не та, что пишет текст.
models-not-offered-no-chat-template = В нём нет шаблона чата, поэтому это не чат-модель, к которой { -brand-name } может обратиться.
models-not-offered-not-gguf = Не файл GGUF: локальный движок запускает только модели GGUF.
models-not-offered-unreadable = Его заголовок не удалось прочитать: { $reason }

settings-retention-title = Что сохраняется
settings-retention-description = Куда ложится результат, что происходит с файлом, из которого он получен, и хранит ли { -brand-name } копию того, что пришло без файла.

settings-retention-beside = Результаты записываются рядом с файлом, как name.cleaned.ext; сам файл никогда не трогается.
settings-retention-into = Результаты записываются в { $folder }; сам файл никогда не трогается.
settings-retention-over = Файл заменяется своим результатом, как только оригинал отложен как name.original.ext — и уже лежащий там оригинал никогда не перезаписывается.
settings-retention-keeps-nothing = То, что пришло без файла — вставленное, перетащенное из браузера — не хранится, как только результат его заменил.
settings-retention-keeps-originals = Оригинал вставленного или перетащенного хранится в { $folder } { $period }, если очистка что-то в нём изменила; результаты — нет.
settings-retention-keeps-results = Результат вставленного или перетащенного хранится в { $folder } { $period }; оригиналы — нет.
settings-retention-keeps-both = Оригинал и результат вставленного или перетащенного хранятся в { $folder } { $period }, если очистка что-то в нём изменила.
settings-retention-pending = Окна следуют этим правилам: очистка берёт их в момент начала, переписывание — в момент постановки в очередь, и результат записывается туда, куда они велели тогда, даже после перезапуска. Командная строка и агенты не читают ни одного из них — командной строке место для результата называют при каждом запуске, а агент получает результат обратно.

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
settings-mcp-description = Агент сможет применять { -layer-a } к собственному тексту и переписывать его назначенным движком по протоколу Model Context Protocol.
settings-mcp-tools = Работают пять инструментов: inspect показывает, что { -layer-a } изменила бы в тексте, clean вносит эти изменения и сообщает о каждом вместе с позицией, inspect_image и clean_image делают то же с PNG, JPEG или WebP — с метаданными и с известными этой версии видимыми метками в пикселях, которые clean_image снимает, если может их доказать; метки, которых не видит глаз, не ищутся и не снимаются, — а rewrite отдаёт текст назначенному движку на переписывание между двумя проходами { -layer-a }, в той же очереди, что и переписывания главного окна, — документ уходит туда, куда его отправляет страница «Движок». Переписывание делается по мере возможностей, и его отчёт говорит, чего оно не устанавливает. Каждая очистка и каждое переписывание — строка в списке главного окна, если вызов не говорит record: false.

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
report-not-established-invisible-pixel-marks = невидимые метки в пикселях картинки — не искались и не снимались

unicode-class-zero-width = символ нулевой ширины
unicode-class-zwj = соединитель нулевой ширины
unicode-class-bidi-control = управляющий символ направления письма
unicode-class-tag-character = символ-тег
unicode-class-variation-selector = селектор варианта
unicode-class-soft-hyphen = мягкий перенос
unicode-class-exotic-space = нетипичный пробел
unicode-class-noncharacter = несимвол
unicode-class-private-use = символ для частного использования
unicode-class-default-ignorable = игнорируемый символ форматирования
unicode-class-homoglyph = буква из другой письменности

confidence-confirmed = подтверждено
confidence-probable = вероятно
confidence-informational = для сведения
confidence-likely-false-positive = скорее всего, не метка

## Строка меню

tray-show = Показать { -brand-name }
tray-panel = Показать панель
tray-clean-clipboard = Очистить буфер обмена — пока нет
tray-unload-model = Выгрузить модель
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

cli-command-inspect = Показывает, что находится в документе или в изображении PNG, JPEG или WebP — в его метаданных и в его пикселях, если там видимая метка, описанная известным профилем, — ничего не изменяя. Невидимые метки в пикселях не ищутся.
cli-command-clean = Только { -layer-a }: детерминированно, проверяемо, без модели. Изображение PNG, JPEG или WebP лишается метаданных ИИ-происхождения; если больше ничего нет, данные изображения сохраняются байт в байт. Доказанная видимая метка удаляется, и тогда изображение записывается заново — JPEG с качеством 95, WebP с потерями — без потерь. Невидимые метки в пикселях остаются.
cli-command-rewrite = { -layer-a }, затем перезапись моделью, затем снова { -layer-a }.
cli-command-models = Управление моделями на этом компьютере: загрузками из каталога и моделями, добавленными вами.
cli-command-models-list = Показать каждую модель каталога, что из неё есть на этой машине и поместится ли она.
cli-command-models-pull = Загрузить модель по идентификатору, докачивая начатый файл.
cli-command-models-verify = Заново посчитать хеш модели целиком: модели из каталога — сверить с каталогом, добавленной вами — с контрольной суммой, записанной при добавлении. Код 1, если не совпадает или модели нет.
cli-command-models-rm = Удалить установленную модель.
cli-command-models-add = Добавить GGUF, которого нет в каталоге: прочитать его заголовок, один раз посчитать хеш и записать под идентификатором. Файл никуда не перемещается и не копируется.
cli-command-models-forget = Забыть добавленную вами модель. Файл никогда не удаляется.
cli-command-audit = Обойти папку и сообщить о каждом текстовом файле в ней, где есть находки, и о каждом PNG, JPEG или WebP, в метаданных которого есть ИИ-происхождение или в пикселях которого видимая метка, для pre-commit-хуков и CI. Невидимые метки в пикселях не ищутся. Код 3, если хоть один файл не удалось прочитать — даже если в других есть находки: проверка с дырой не полна.

cli-arg-path-or-stdin = Файл для чтения или `-` для стандартного ввода.
cli-arg-path = Файл для чтения.
cli-arg-out = Файл вывода или `-` для стандартного вывода. По умолчанию `<name>.cleaned.<ext>` рядом с исходным, а при чтении из стандартного ввода — стандартный вывод; запись поверх исходника требует явного флага и никогда не делается по умолчанию.
cli-arg-nfkc = Применить нормализацию NFKC (по умолчанию выключена — она меняет больше, чем метки происхождения).
cli-arg-aggressive = Заменять также букву из другой письменности внутри слова (омоглиф). О таких буквах сообщается в любом случае; выше доля ложных срабатываний, поэтому только по требованию.
cli-arg-json = Машиночитаемый JSON вместо обычного текста.
cli-arg-tactic = Как просить модель: paraphrase (по умолчанию), humanize или back_translate. structural есть только в приложении, после подтверждения; code в этой версии нет.
cli-arg-candidates = Число кандидатов на фрагмент. Без него решает тот, кто переписывает: 1 для модели только на процессоре этого компьютера, 2 для модели на видеокарте или для адреса.
cli-arg-rounds = Наибольшее число проходов на фрагмент. Без него — до 2, и второй только если ни один кандидат первого не прошёл.
cli-arg-intensity = Насколько paraphrase и humanize могут уходить от исходных слов: light, moderate (по умолчанию) или strong.
cli-arg-format = Что за текст: plain, markdown или html. Без него — то, чем оказался файл; для всего прочего plain. В markdown и html переписывается только проза.
cli-arg-prompts = JSON-файл строк шаблонов поверх сохранённых приложением: ключ строки и либо её сохранённое значение, либо текст шаблона. Шаблон, нарушающий правило, останавливает запуск до того, как что-либо отправлено.
cli-arg-seed = Базовое зерно. Без него каждый запуск получает новое, и повторный запуск даёт другую перезапись; зерно из отчёта, переданное обратно, повторяет запуск на модели этого компьютера.
cli-arg-id = Идентификатор модели: из каталога или добавленной вами (user-…).
cli-arg-model-path = Файл GGUF, который нужно добавить.
cli-arg-name = Как её назвать. По умолчанию — имя, которое файл даёт себе сам.
cli-arg-role = Для чего она: rewrite — единственное назначение, для которого эта версия добавляет модели.
cli-arg-ctx = Окно контекста, с которым она загружается, в токенах.
cli-arg-dir = Каталог для обхода.
cli-arg-sarif = Вывод в формате SARIF, для панелей сканирования кода.
cli-arg-in-place = Заменить файл его очищенным текстом или изображением. Оригинал сначала откладывается рядом как `<name>.original.<ext>`, а уже лежащий там оригинал никогда не перезаписывается: запуск тогда отказывается. Если менять нечего, ничего не трогается.
cli-arg-all-metadata = Для изображения: удалить все блоки метаданных, а не только ИИ-происхождение, — и данные камеры (EXIF вместе с ориентацией, по которой картинка может показываться прямо), и XMP, IPTC, комментарии. Цветовые профили остаются в любом случае: без них картинка выглядит иначе. Не для текста.
cli-arg-no-original = Вместе с --in-place: не сохранять копию оригинала — для файлов под контролем версий, где копией служит история.
cli-arg-language = Язык сообщений и справки, тег BCP-47, например de или ru. Имеет приоритет над WIPEMARK_LANG, языком, сохранённым в настройках приложения, и операционной системой — именно в этом порядке.

cli-report-stdin = стандартный ввод
cli-report-none = { $source }: ни одного из символов, которые ищет эта версия, не найдено.
cli-report-noted = { $source }: { $count ->
        [1] найден один символ, и он, скорее всего, не метка.
        [one] найден { $count } символ, и ни один из них, скорее всего, не метка.
        [few] найдено { $count } символа, и ни один из них, скорее всего, не метка.
       *[other] найдено { $count } символов, и ни один из них, скорее всего, не метка.
    }
cli-report-suspicious = { $source }: { $count ->
        [1] найден один символ, и он похож на метку.
        [one] найден { $count } символ, и хотя бы один из них похож на метку.
        [few] найдено { $count } символа, и хотя бы один из них похож на метку.
       *[other] найдено { $count } символов, и хотя бы один из них похож на метку.
    }
cli-report-would-remove = Будет удалено:
cli-report-would-replace = Будет заменено:
cli-report-would-keep = Будет оставлено:
cli-report-removed = Удалено:
cli-report-replaced = Заменено:
cli-report-kept = Оставлено:
cli-report-row = { $character } · { $class } · { $confidence } · { $count ->
        [1] один раз, на байте { $positions }
        [one] { $count } раз, на байтах { $positions }
        [few] { $count } раза, на байтах { $positions }
       *[other] { $count } раз, на байтах { $positions }
    }
cli-report-more = { $shown } и ещё { $more }
cli-report-homoglyphs-kept = Найдены буквы из другой письменности, и они не заменены: clean заменяет их только с --aggressive, каждую — на похожую букву той письменности, которой написано слово.
cli-report-offsets = Смещения в байтах считаются по тексту в UTF-8; на входе был { $encoding }.
cli-report-unicode = Проверено по Unicode { $version }.
cli-clean-written = Результат — в { $path }.
cli-clean-untouched = Сам файл { $source } не изменён.
cli-clean-nfkc = Применена и нормализация NFKC; всё, что она обнажила, вычищено следующими проходами, и --json считает это без позиций.
cli-clean-later = { $count ->
        [1] Ещё один символ, который обнажила NFKC, удалён или заменён следующим проходом; позиции во входном тексте у него нет, поэтому в строках выше он может значиться оставленным.
        [one] Ещё { $count } символ, которые обнажила NFKC, удалены или заменены следующими проходами; позиций во входном тексте у них нет, поэтому в строках выше они могут значиться оставленными.
        [few] Ещё { $count } символа, которые обнажила NFKC, удалены или заменены следующими проходами; позиций во входном тексте у них нет, поэтому в строках выше они могут значиться оставленными.
       *[other] Ещё { $count } символов, которые обнажила NFKC, удалены или заменены следующими проходами; позиций во входном тексте у них нет, поэтому в строках выше они могут значиться оставленными.
    }
cli-no-such-file = { $path } не существует.
cli-is-a-folder = { $path } — это папка. inspect и clean читают один файл или стандартный ввод; папку обходит audit.
cli-out-is-a-folder = --out указывает на папку, { $path }. Нужно имя файла.
cli-out-is-input = --out указывает на читаемый файл, { $path }. Чтобы заменить файл, есть --in-place: он сначала откладывает оригинал.
cli-unreadable = { $path } не удалось прочитать: { $reason }. Не прочитано — не значит чисто.
cli-not-text = { $path }: внутри { $format }, а не текст, — { -layer-a } здесь неприменима. Не прочитано — не значит чисто.
cli-not-text-unknown = { $path } — не текст ни в одной из кодировок, которые читает эта версия. Не прочитано — не значит чисто.
cli-unnamed-encoding = { $path } — текст в 8-битной кодировке, которую эта версия не называет. Сохраните его в UTF-8 и запустите снова; до тех пор он не прочитан, а не прочитано — не значит чисто.
cli-invalid-encoding = { $path }: на байте { $offset } — недопустимая последовательность { $encoding }. Не прочитано — не значит чисто.
cli-name-disagrees = { $path }: имя обещает { $named }, а внутри — { $found }; прочитано по содержимому.
cli-write-failed = { $path } не удалось записать: { $reason }. Результат не сохранён.

cli-in-place-stdin = --in-place заменяет файл, а стандартный ввод — не файл. Укажите файл или запишите результат через --out.
cli-in-place-link = { $path } — символическая ссылка. --in-place заменяет файлы, а не ссылки; запустите его на файле, на который она указывает.
cli-in-place-original-exists = { $original } уже существует, а отложенный ранее оригинал никогда не перезаписывается. { $path } не изменён. Переместите { $original } или запустите с --no-original.
cli-in-place-set-aside-failed = { $path } не удалось отложить как { $original }: { $reason }. Ничего не изменено.
cli-in-place-write-failed = Очищенный текст не удалось записать в { $path }: { $reason }. { $path } не изменён.
cli-in-place-stranded = Очищенный текст не удалось записать в { $path } ({ $reason }), и оригинал не удалось вернуть на место ({ $restore }). Оригинал сейчас лежит в { $original }.
cli-in-place-original = Оригинал отложен как { $original }.
cli-in-place-no-original = Копия оригинала не сохранена (--no-original).
cli-in-place-unchanged = { $source }: менять было нечего, поэтому файл не тронут и оригинал не откладывался.

cli-audit-not-a-folder = { $path } — не папка. audit обходит папку; inspect и clean читают один файл.
cli-audit-file = { $path }: { $count ->
        [one] { $count } находка
        [few] { $count } находки
        [many] { $count } находок
       *[other] { $count } находки
    } ({ $classes })
cli-audit-summary = { $root }: проверено { $scanned } · с находками { $findings } · пропущено { $skipped } · не прочитано { $unreadable }
cli-audit-unreadable-title = Не удалось прочитать, а значит, чистота не показана:
cli-audit-image-visible = { $path }: { $count ->
    [one] видимая метка
    [few] { $count } видимые метки
   *[other] { $count } видимых меток
} ({ $profiles })
cli-audit-image = { $path }: { $container }, { $count ->
        [1] один блок
        [one] { $count } блок
        [few] { $count } блока
       *[other] { $count } блоков
    } с ИИ-происхождением ({ $kinds })

cli-image-none = { $source }: { $container }, блоков метаданных нет.
cli-image-noted = { $source }: { $container }, { $count ->
        [1] один блок метаданных, и он не говорит об ИИ-происхождении.
        [one] { $count } блок метаданных, и ни один из них не говорит об ИИ-происхождении.
        [few] { $count } блока метаданных, и ни один из них не говорит об ИИ-происхождении.
       *[other] { $count } блоков метаданных, и ни один из них не говорит об ИИ-происхождении.
    }
cli-image-ai = { $source }: { $container }, { $count ->
        [1] один блок метаданных
        [one] { $count } блок метаданных
        [few] { $count } блока метаданных
       *[other] { $count } блоков метаданных
    }, { $ai ->
        [1] один из них говорит об ИИ-происхождении.
       *[other] из них об ИИ-происхождении говорят: { $ai }.
    }
cli-image-row = { $where } · { $kind } · с байта { $offset } · { $length ->
        [one] { $size } байт
        [few] { $size } байта
       *[other] { $size } байт
    }
cli-image-evidence = { $signal }, в { $field }: { $matched }
cli-image-evidence-generator = { $signal } ({ $generator }), в { $field }: { $matched }
cli-image-rendering = Сведения о цвете (ICC-профиль, гамма, sRGB) сохраняются при любых флагах: без них картинка выглядела бы иначе.
cli-image-exif-removed = Блок EXIF называл генератор изображений, поэтому удалён целиком, вместе с данными камеры.
cli-image-all-metadata = --all-metadata удалил и данные камеры.
cli-image-orientation-removed = Поворот картинки был записан в удалённых данных камеры: программа, которая ставила её правильно, теперь покажет её так, как она хранится, — повёрнутой или отражённой.
cli-image-pixels = Пиксели проверены на видимые метки, которые знает эта версия. Метки, которых не видит глаз, не ищутся, и ничто здесь не утверждает, что их в картинке нет.
cli-image-visible-title = Видимые метки
cli-image-visible-none = Видимых меток, известных этой версии, в пикселях не найдено.
cli-image-visible-row = { $profile } ({ $vendor }, { $product }) · { $width }×{ $height } в точке { $x },{ $y } · { $placed }
cli-image-visible-placed-row = на своём известном месте
cli-image-visible-placed-searched = найдена поиском
cli-image-visible-proved = доказана: корреляция { $ncc }, сила { $gain }, доля контура { $ratio }
cli-image-visible-refused = видна, но не доказана: { $reason }
cli-image-refusal-transparent = картинка под меткой не непрозрачна
cli-image-refusal-opaque = метка непрозрачна целиком ({ $holes } пикселей), и под ней ничего не восстановить
cli-image-refusal-gain = её края исчезают при силе { $k }, а не при её собственной
cli-image-refusal-edges = после снятия от её контура осталось бы { $ratio }
cli-image-refusal-out-of-range = снятие вывело бы { $share } значений за допустимый диапазон
cli-image-visible-restored = { $profile }: восстановлено пикселей: { $changed }.
cli-image-visible-exact = Восстановленные пиксели совпадают с исходными с точностью до одного уровня.
cli-image-visible-inexact = Картинка хранилась с потерями, поэтому восстановление настолько близко, насколько позволяют сохранённые значения, но не точно.
cli-image-visible-holes = Пиксели под непрозрачной частью метки ({ $holes }) восстановить нельзя; они оставлены как были.
cli-image-visible-outline = Вдоль края метки остался её контур — в среднем { $levels } уровня от картинки вокруг в самом отличающемся цветовом канале, { $share } % контура, — больше, чем допускает эта версия, поэтому метка считается оставшейся в результате.
cli-image-visible-texture = Вдоль края метки осталась зернистость — по 95-му процентилю её пиксели отличаются от соседних на { $levels } уровня, а в картинке вокруг на { $around }, — больше, чем допускает эта версия, поэтому метка считается оставшейся в результате.
cli-image-visible-clamped = { $clamped ->
        [one] При обращении смешивания { $clamped } значение вышло за пределы диапазона и было обрезано, поэтому восстановление не точное.
        [few] При обращении смешивания { $clamped } значения вышли за пределы диапазона и были обрезаны, поэтому восстановление не точное.
       *[other] При обращении смешивания { $clamped } значений вышли за пределы диапазона и были обрезаны, поэтому восстановление не точное.
    }
cli-image-visible-fitted = Карта прозрачности метки измерена по настоящим выходам, а не взята у производителя, поэтому точность восстановления не заявляется.
cli-image-visible-resampled = Метка стояла не там и не того размера, для которых нарисована её карта; карта пересчитана, поэтому точность восстановления не заявляется.
cli-image-visible-searched = Метка найдена поиском, не на том месте, которое называет её профиль, поэтому точность восстановления не заявляется.
cli-image-visible-residual = Вдоль слабого края восстановленная метка в среднем отличается от картинки вокруг на { $levels } уровня — в самом отличающемся цветовом канале.
cli-image-visible-left = Видимая метка найдена и осталась в результате.
cli-image-visible-not-restorable = Такие картинки (JPEG в CMYK) эта версия обратно не записывает, поэтому метка оставлена.
cli-image-visible-not-examined-animated = Анимированная картинка: её кадры на видимые метки не проверялись, только метаданные. Непроверенное — не значит чистое.
cli-image-visible-not-examined-catalogue = Каталог видимых меток этой сборки не загрузился, поэтому пиксели не проверялись. Не проверено — не значит чисто.
cli-image-visible-not-examined-decode = Пиксели картинки не удалось декодировать, поэтому на видимые метки они не проверялись. Не прочитано — не значит чисто.
cli-image-encoded-jpeg = Картинка перекодирована в JPEG с качеством { $quality }.
cli-image-encoded-webp = Картинка записана как WebP без потерь.
cli-image-encoded-webp-from-lossy = Картинка была WebP с потерями и записана как WebP без потерь: файл больше, новых потерь не добавлено.
cli-image-encoded-png = PNG записан заново с восстановленными пикселями.
cli-image-encoded-png-colour = Тип цвета PNG изменился: восстановленные цвета не уложились в исходный.
cli-image-encoded-png-interlace = PNG записан без чересстрочности.
cli-image-proof-failed = { $path }: результат не прошёл собственную проверку, поэтому ничего не записано. Это ошибка этой версии.
cli-image-encode-failed = { $path }: восстановленную картинку не удалось записать обратно. Ничего не записано.

cli-image-not-yet = { $path }: изображения { $container } в этой версии пока не поддерживаются. Метаданные не прочитаны, ничего не записано.
cli-image-unknown = { $path }: эти байты — не изображение, которое открывает эта версия. Ничего не записано.
cli-image-multi-picture = В { $path } после первой картинки лежат ещё (MPF), и это удаление оставило бы их указатель неверным: метаданные после указателя сдвинули бы картинки, или сам указатель не удалось прочитать, чтобы исправить. Ничего не записано.
cli-image-reframe = { $path }: пиксели картинки изменились, а записать их обратно в этот файл эта версия не может: он анимированный или содержит часть, которой эта версия не знает. Ничего не записано.
cli-image-malformed = { $path } — не тот файл { $container }, который может прочитать эта версия: { $defect }, на байте { $offset }. Не прочитано — не значит чисто.
cli-image-text-flag = { $path } — изображение ({ $container }), а { $flag } — флаг для текста. Ничего не записано.
cli-image-all-metadata-text = { $path } — не изображение, а --all-metadata — флаг для изображений. Ничего не записано.
cli-image-to-terminal = Очищенное изображение ушло бы в терминал. Запишите его в файл через -o или перенаправьте стандартный вывод.
cli-image-json-stdout = --json пишет ответ в стандартный вывод, и изображение пошло бы туда же; запишите изображение в файл через -o.
cli-image-still-marked = { $path }: в результате остались бы метаданные ИИ-происхождения, поэтому он не записан. Удалить удалось не всё.

image-kind-c2pa = манифест C2PA
image-kind-exif = EXIF
image-kind-xmp = XMP
image-kind-iptc = IPTC
image-kind-generator-parameters = параметры генератора
image-kind-other-text = текст
image-kind-rendering = сведения о цвете
image-kind-other = прочие метаданные

image-signal-c2pa-manifest = манифест C2PA
image-signal-c2pa-reference = ссылка на манифест C2PA
image-signal-digital-source-type = тип цифрового источника IPTC, называющий модель или алгоритм
image-signal-generator-key = текстовый ключ, который пишет генератор изображений
image-signal-generator-text = подпись генератора изображений

image-defect-truncated = он обрывается посреди блока
image-defect-bad-signature = его сигнатура не на своём месте
image-defect-header-not-first = его заголовок — не первый блок
image-defect-no-end = в нём нет маркера конца
image-defect-bad-length = у блока длина, какой не бывает
image-defect-bad-chunk-type = имя блока — не четыре буквы
image-defect-bad-marker = на месте маркера стоит другой байт
image-defect-riff-size = его заголовок RIFF обещает больше байт, чем есть в файле
image-defect-bad-text = текстовый блок устроен не как текстовый
image-defect-inflate = сжатый текст не распаковывается
image-defect-inflate-limit = сжатый текст распаковывается за предел, который читает эта версия

cli-models-folder = Папка моделей: { $path }
cli-models-entry = { $id } · { $name } · { $roles } · { $size } · { $state } · { $fit }
cli-models-chosen = выбрана для перезаписи
cli-models-found-at = найдена здесь: { $path }
cli-models-foreign-at = { $path } — чужой файл с этим именем, он остаётся как есть
cli-models-foreign-part-at = { $path } — частичный файл, о котором у { -brand-name } нет записи; он остаётся как есть
cli-models-state-present = есть на этой машине, совпадает с каталогом
cli-models-state-absent = не загружена
cli-models-state-partial = загружена частично ({ $percent } %), pull докачает
cli-models-state-mismatch = есть на этой машине, но не совпадает с каталогом
cli-models-fit-fits = помещается на этой машине
cli-models-fit-tight = помещается на этой машине, но впритык
cli-models-fit-too-big = нужно на { $short } МБ больше памяти, чем есть на этой машине
cli-models-fit-unknown = поместится ли на этой машине, неизвестно
cli-models-size = { $gigabytes } ГБ
cli-models-others-title = Также в этой папке, не из каталога — только перечислены, не проверены, и ничто их не загружает:
cli-models-folder-unreadable = Папку моделей { $path } не удалось прочитать: { $reason }.
cli-models-unknown-id = { $id } нет в каталоге. Его идентификаторы: { $ids }.
cli-models-pull-present = { $id } уже есть на этой машине и совпадает с каталогом: { $path }
cli-models-pull-progress = { $id }: { $done } из { $total } МБ ({ $percent } %)
cli-models-pull-done = { $id } загружена и совпадает с каталогом: { $path }
cli-models-pull-cancelled = { $id }: отменено. Загруженное сохранено; запустите pull ещё раз, чтобы докачать.
cli-models-pull-mismatch = { $id }: { $file } не совпадает с каталогом (ожидался sha256 { $expected }, получен { $actual }), поэтому он удалён вместе с частичным файлом. Ничего не установлено.
cli-models-pull-no-room = { $id } нужно { $need } МБ на томе с { $path }, а свободно { $free } МБ. Ничего не загружено.
cli-models-pull-failed = { $id } не удалось загрузить: { $reason }. Загруженное до сих пор сохранено; запустите pull ещё раз, чтобы докачать.
cli-models-pull-occupied = { $id } не загружена: { $path } — файл, который { -brand-name } не загружал, поэтому он оставлен как есть и ничего не скачано. Уберите его и запустите pull ещё раз.
cli-models-pull-occupied-part = { $id } не загружена: { $path } — частичный файл, о котором у { -brand-name } нет записи, поэтому он оставлен как есть и ничего не скачано. Уберите его и запустите pull ещё раз.
cli-models-verify-ok = { $id } совпадает с каталогом: хеш каждого файла посчитан целиком.
cli-models-verify-absent = { $id } нет на этой машине ({ $file } отсутствует), значит, она не совпадает с каталогом.
cli-models-verify-mismatch = { $id }: { $file } не совпадает с каталогом (ожидался sha256 { $expected }, получен { $actual }). pull загрузит его заново.
cli-models-verify-unreadable = { $id }: { $file } не удалось прочитать: { $reason }. Не прочитано — не проверено.
cli-models-rm-removed = { $id } удалена из { $path }.
cli-models-rm-absent = { $id } не было на этой машине; ничего не удалено.
cli-models-rm-found = { $id } лежит здесь: { $path }, куда { -brand-name } её не загружал; ничего не удалено.
cli-models-rm-found-part = У { $id } есть частичный файл { $path }, о котором у { -brand-name } нет записи; ничего не удалено.
cli-models-rm-chosen = Это была модель, выбранная для перезаписи: приложение будет показывать, что модель не выбрана, пока не выбрать другую. Эта команда настройку не меняет.
cli-models-rm-failed = { $id } не удалось удалить из { $path }: { $reason }.
cli-models-user-entry = { $id } · { $name } · { $roles } · { $size } · { $state } · { $fit } · добавлена вами: { $path }
cli-models-state-user-present = на этом компьютере, совпадает с контрольной суммой, записанной при добавлении
cli-models-state-user-changed = изменилась после добавления, не загружается, пока её не добавят заново
cli-models-state-user-missing = файла больше нет по этому пути
cli-models-state-user-unreadable = не удалось прочитать: { $reason }
cli-models-add-done = { $id }: { $path } добавлен как { $name }. Его контрольная сумма записана; { -brand-name } не может поручиться за то, что это за модель.
cli-models-add-again = { $id }: { $path } добавлен заново как { $name }; записана контрольная сумма файла в нынешнем виде.
cli-models-add-unreadable = { $path } не удалось добавить: его заголовок не прочитан ({ $reason }).
cli-models-add-not-offered = { $path } не добавлен: { $why }
cli-models-add-no-database = Базы данных { -brand-name } по пути { $path } ещё нет: откройте приложение один раз, затем повторите команду. Ничего не добавлено.
cli-models-add-database = В базу данных приложения по пути { $path } эта команда писать не может: { $reason }. Откройте приложение один раз, затем повторите команду. Ничего не добавлено.
cli-models-add-role = { $role } — не назначение, для которого эта версия добавляет модели; она добавляет их для rewrite.
cli-models-add-ctx = { $ctx } токенов — вне того, что принимает эта модель: от { $min } до { $max }.
cli-models-add-name = Название пустое, длиннее { $max } символов или содержит управляющий символ.
cli-models-add-hash-failed = { $path } не удалось прочитать целиком: { $reason }. Ничего не добавлено.
cli-models-add-progress = { $path }: прочитано { $done } из { $total } МБ ({ $percent } %)
cli-models-forget-done = { $id } ({ $name }) забыта. Файл остаётся по пути { $path }: { -brand-name } никогда не удаляет файл, который не скачивал.
cli-models-forget-catalogue = { $id } — модель из каталога, а не добавленная вами: её загрузку удаляет rm.
cli-models-user-not-downloadable = { $id } — модель, добавленная вами из { $path }: скачивать или удалять нечего. Забудьте её, чтобы больше не использовать.
cli-models-unknown-model = { $id } нет ни в каталоге, ни среди добавленных вами моделей. Идентификаторы: { $ids }.
cli-models-verify-user-ok = { $id } совпадает с контрольной суммой, записанной при добавлении: хеш файла посчитан целиком.
cli-models-verify-user-changed = { $id }: { $path } — не тот файл, что был добавлен: контрольная сумма другая. Добавьте его заново, чтобы использовать в нынешнем виде.
cli-models-verify-user-missing = { $id }: файла { $path } больше нет.
cli-models-verify-user-unreadable = { $id }: { $path } не удалось прочитать: { $reason }. Не прочитано — значит не проверено.

cli-unknown-language = неизвестный язык `{ $requested }`, выполняется откат. Доступны: { $available }

cli-rewrite-tactic-structural = Тактика structural переписывает документ по его плану и есть только в приложении, после подтверждения. Ничего не переписано.
cli-rewrite-tactic-code = Тактики code в этой версии нет. Ничего не переписано.
cli-rewrite-needs-app-endpoint = Переписывание настроено на сервер, а командная строка обращается к нему только через запущенное приложение { -brand-name }. Запустите его и повторите — или на его странице «Движок» поручите переписывание модели на этом компьютере. Ничего не переписано.
cli-rewrite-needs-app-fallback = Модели, выбранной для переписывания, нет на этом компьютере, а сервер, назначенный отвечать вместо неё, доступен только через запущенное приложение { -brand-name }. Скачайте модель или запустите приложение и повторите. Ничего не переписано.
cli-rewrite-no-model = Модель этого компьютера для переписывания не выбрана. Скачайте её командой wipemark-cli models pull и выберите на странице «Модели» приложения — или запустите приложение с назначенным сервером. Ничего не переписано.
cli-rewrite-model-not-here = Модели, выбранной для переписывания, { $id }, нет на этом компьютере целиком; её скачивает wipemark-cli models pull { $id }. Ничего не переписано.
cli-rewrite-added-model-not-here = Модель, выбранная для переписывания, { $id }, добавлена вами, и её файл изменился или пропал; что именно — скажет wipemark-cli models verify { $id }. Ничего не переписано.
cli-rewrite-unavailable = Ничего не переписано: { $reason }
cli-rewrite-failed = Ничего не переписано: задача не удалась ({ $reason }).
cli-rewrite-cancelled = Отменено. Ничего не записано.
cli-rewrite-lost = Приложение перестало отвечать до того, как вернулась перезапись. Ничего не записано; возможно, оно ещё доделывает задачу.
cli-rewrite-app-refused = Запущенное приложение не переписало: { $reason }
cli-rewrite-served-app = Переписано запущенным приложением { -brand-name }, на его назначенном движке.
cli-rewrite-served-here = Переписано этой командой, на модели, выбранной для переписывания.
cli-rewrite-price = { $calls ->
        [1] Модель ответит не больше одного раза, примерно { $tokens } токенов.
       *[other] Модель ответит не больше { $calls } раз — { $expected }, если каждый абзац пройдёт сразу, — примерно { $tokens } токенов.
    }
cli-rewrite-progress = абзац { $chunk } из { $chunks } · кандидат { $candidate } из { $candidates } · проход { $round } из { $rounds }
cli-rewrite-summary = { $chunks ->
        [0] Прозы для переписывания в нём нет; код, заголовки и разметка остаются как есть.
        [1] Переписано абзацев: { $rewritten } из одного.
       *[other] Переписано абзацев: { $rewritten } из { $chunks }.
    }
cli-rewrite-kept = Абзацев с очищенным оригиналом: { $kept } — ни один кандидат для них не прошёл проверки. Переписано не всё, поэтому код выхода 3.
cli-rewrite-attempts = Кандидатов, написанных моделью: { $attempts }; отклонено: { $rejected }.
cli-rewrite-best-effort = Переписывание делается по мере возможностей: оно меняет формулировки, а чего оно не устанавливает — перечислено ниже.
cli-rewrite-seed = Базовое зерно { $seed }; --seed { $seed } повторяет этот запуск на модели этого компьютера.
cli-prompts-unreadable = Файл шаблонов { $path } не удалось прочитать: { $reason }. Ничего не переписано.
cli-prompts-not-rows = Файл шаблонов { $path } — не JSON-объект строк шаблонов ({ $reason }). Ничего не переписано.
cli-prompts-unknown-row = Файл шаблонов { $path } называет { $key }, а такой строки шаблона в этой версии нет. Ничего не переписано.
cli-prompts-invalid = Шаблон { $key } в { $path } нарушает правило { $rule }. Ничего не переписано.

## The windows clean (E7)

toolbar-clean-all = Очистить всё
toolbar-clean-all-tooltip = Очистить каждую строку, которая ждёт и может быть очищена, по одной, в порядке поступления. Неактивна, пока таких нет.
queue-column-status = Состояние
queue-status-waiting = Не начато
queue-status-waiting-tooltip = С ним ещё ничего не просили сделать, и ничего не произойдёт, пока не попросите: «Очистить» или «Переписать» в его строке, либо «Очистить всё» и «Переписать всё» на панели. «Настройки › Общие › Обрабатывать поступившее» может делать это сразу при поступлении.
queue-status-unable = Не очищается
queue-status-queued = В очереди на очистку
queue-status-queued-tooltip = Ждёт, пока закончится очистка перед ним: очищается одно за раз, в том порядке, в каком попросили.
queue-status-cleaning = Очищается…
queue-status-cleaning-tooltip = Очищается сейчас. Пока не закончится, ничего не записывается.
queue-status-nothing-found = Меток не найдено
queue-status-cleaned = Очищено
queue-status-partly = Очищено частично
queue-status-not-cleaned = Не очищено
queue-status-failed = Очистка не удалась
queue-action-clean = Очистить
queue-action-clean-done = Это уже очищено.
queue-action-clean-busy = Это уже стоит в очереди на очистку.
queue-action-open-result = Открыть результат
queue-action-reveal-result = Показать результат в папке
queue-action-copy-result = Скопировать результат
queue-action-replace = Заменить существующий результат
queue-went-written = Записано как { $name }
queue-went-replaced = Записано поверх существующего { $name }
queue-went-in-place = Записано на место файла
queue-went-set-aside = Оригинал отложен как { $name }
queue-went-kept = Сохранено в { $folder }
queue-went-as-text = Очищенный текст готов: «Скопировать результат» есть в меню «Действия».
queue-went-nothing = Ничего не записано.
status-cleaning = Очистка: { $current } из { $total } { $total ->
        [one] элемента
        [few] элементов
        [many] элементов
       *[other] элемента
    }
clean-said-nothing-found = Проверено: невидимых меток и следов ИИ не найдено, удалять было нечего, поэтому результат не записан. Сам файл найден и прочитан.
clean-said-cleaned-text = { $count ->
        [one] Удалён или заменён { $count } символ.
        [few] Удалено или заменено { $count } символа.
        [many] Удалено или заменено { $count } символов.
       *[other] Удалено или заменено { $count } символа.
    }
clean-said-cleaned-picture = То, что помечало картинку как сделанную ИИ, удалено.
clean-said-partly-kept = Найдено то, что при настройках по умолчанию оставляется, — буква другого алфавита, похожая на латинскую, — поэтому ничего не изменено.
clean-said-partly-mark = Видимая метка осталась на картинке: целиком её снять не удалось.
clean-said-partly-animated = Кадры анимированной картинки на видимую метку не проверяются, так что метка там не найдена и не исключена.
clean-said-partly-unexamined = Пиксели картинки проверить не удалось, так что видимая метка там не найдена и не исключена.
clean-refused-not-yet = Картинки { $format } в этой версии пока не читаются.
clean-refused-folder = Папка не очищается как одно целое; перетащите файлы из неё.
clean-refused-kind = Ни очистка текста, ни очистка картинок такое не читают: { $what }.
clean-refused-unnamed-encoding = Её символы в кодировке, которую не удалось назвать, а кодировка никогда не угадывается.
clean-refused-unread = По содержимому ничего установить не удалось, а одного имени для очистки мало.
clean-refused-too-big = При размере { $size } это больше, чем очищает окно; предел — { $limit }.
clean-refused-unreadable = Прочитать не удалось.
clean-refused-undecodable = Это не корректный { $encoding } на байте { $offset }, поэтому ничего не изменено.
clean-refused-picture-unknown = Это не картинка, которую эта версия умеет читать.
clean-refused-picture-malformed = Это не файл { $format }, который эта версия умеет читать: он повреждён на байте { $offset }. Не прочитано — не значит чисто.
clean-refused-picture-unsupported = В нём есть то, что эта версия в { $format } не поддерживает, на байте { $offset }.
clean-refused-picture-decode = Пиксели не удалось декодировать, поэтому картинка не очищена.
clean-refused-picture-encode = Восстановленную картинку не удалось записать обратно, поэтому ничего не записано.
clean-refused-picture-proof = Результат не прошёл собственную проверку, поэтому ничего не записано. Это ошибка этой версии.
clean-refused-still-marked = В результате остались бы метаданные о происхождении от ИИ, поэтому он не записан.
clean-refused-exists = { $name } уже существует и оставлен как есть. Чтобы записать поверх, выберите «Заменить существующий результат» в меню «Действия».
clean-refused-original-exists = { $name } уже существует: отложенный раньше оригинал никогда не перезаписывается, поэтому ничего не изменено.
clean-refused-link = { $name } — символическая ссылка, а «Вместо файла» заменяет файл, а не ссылку, поэтому ничего не изменено. Очистите файл, на который она указывает, или выберите для результатов другое место на странице «Хранение».
clean-refused-same-file = Результат лёг бы на сам файл, поэтому ничего не записано.
clean-refused-nowhere = При выборе на странице «Хранение» этому результату некуда лечь.
clean-failed-write = Не удалось записать { $path } ({ $error }). Больше ничего не изменено.
clean-failed-set-aside = Файл не удалось отложить как { $name } ({ $error }), поэтому ничего не изменено.
clean-failed-stranded = Результат не удалось записать, а оригинал не удалось вернуть на место: он лежит в { $path } ({ $error }).
clean-failed-panicked = Очистка остановилась на ошибке этой версии, не успев закончить; ошибка записана в журнал. Прежде чем очищать снова, посмотрите туда, куда лёг бы результат.
queue-action-report = Отчёт…
window-report-title = Отчёт · { $name }
window-report-arrived = Что поступило
window-report-happened = Что произошло
window-report-verifiable = Проверяемо
window-report-best-effort = В меру возможного
window-report-result = Результат: { $path }
window-report-original = Оригинал отложен: { $path }
window-report-kept = Сохранённые копии: { $path }
window-report-finding = { $codepoint } { $name } · { $class } · { $confidence } · { $count ->
        [one] { $count } раз
        [few] { $count } раза
        [many] { $count } раз
       *[other] { $count } раза
    }
window-report-removed-none = Слой очистки ничего не удалил из этого текста.
window-report-normalized = { $count ->
        [one] Нормализован { $count } символ.
        [few] Нормализовано { $count } символа.
        [many] Нормализовано { $count } символов.
       *[other] Нормализовано { $count } символа.
    }
window-report-kept-homoglyph = Оставлено при настройках по умолчанию: буква другого алфавита заменяется только агрессивной очисткой, а окно её не запускает.
window-report-kept-in-place = Оставлено там, где оно нужно: внутри эмодзи или письменности, которой оно требуется.
window-report-picture-checked = Результат прочитан ещё раз: метаданных о происхождении от ИИ в нём не осталось.
window-report-picture-still = При повторном чтении в результате оставались метаданные о происхождении от ИИ.
window-report-shelf-empty = Для этой очистки здесь ничего нет.
window-report-not-read = Это не было прочитано, поэтому здесь ничего не проверено.
window-report-copy-json = Скопировать JSON
window-report-copy-markdown = Скопировать как Markdown
window-report-close = Закрыть
window-report-copied = Скопировано.

panel-looking = Смотрю…
panel-found-text = Удалить или заменить: { $count } { $count ->
        [one] символ
        [few] символа
        [many] символов
       *[other] символа
    }.
panel-found-text-nothing = Удалять нечего.
panel-found-text-kept = Удалять нечего; буква другого алфавита, похожая на латинскую, при настройках по умолчанию остаётся.
panel-found-picture-both = Метаданные ИИ и видимая метка.
panel-found-picture-metadata = Метаданные ИИ.
panel-found-picture-mark = Видимая метка.
panel-found-picture-nothing = Ничего не найдено ни в метаданных, ни среди видимых меток, известных этой версии.
panel-found-not-examined = { $metadata ->
        [yes] Метаданные ИИ есть;
       *[no] Метаданных ИИ нет;
    } { $why ->
        [animated] кадры анимированной картинки на видимую метку не проверяются.
        [catalogue] каталог видимых меток не загрузился, поэтому пиксели не проверены.
       *[decode] пиксели не удалось декодировать, поэтому они не проверены.
    }
panel-clean = Очистить
panel-clean-tooltip = Очистить брошенное сюда, по одному; результат каждого ложится туда, куда велит страница «Хранение». Неактивна, пока идёт очистка или когда здесь больше нечего очищать.
panel-cleaning = Очищаю…

## E4-6b
##
## Окна переписывают. «Переписать» в строке и «Переписать всё» на панели
## ставят документ в единую очередь переписываний приложения — окна, агента
## и командной строки — на движке, который сейчас дежурит; «Переписать всё»
## сначала называет цену. Таблица — это журнал: каждый переданный документ
## — строка со своим состоянием, и строки переживают перезапуск. $reason —
## идентификатор или слова движка, не переводятся; $model — собственное имя
## модели; $host — адрес сервера.
queue-column-process = Обработка
queue-action-rewrite = Переписать
queue-action-rewrite-busy = Уже в очереди переписываний или переписывается.
queue-action-rewrite-cleaning = Сейчас очищается; перепишите, когда очистка закончится.
queue-action-rewrite-not-text = Переписывается только текст; картинка очищается.
queue-action-clean-rewriting = Сейчас переписывается; очистка сейчас соревновалась бы с переписыванием за тот же файл.
queue-action-not-kept = Пришло без файла, и текст не сохранён, поэтому обработать снова нельзя.
queue-action-cancel = Отменить переписывание
queue-action-remove = Убрать из списка
queue-status-rewrite-queued = В очереди на переписывание
queue-status-rewrite-queued-tooltip = Ждёт своей очереди: переписывается один документ за раз, в порядке просьб, кто бы ни просил.
queue-status-held = Ждёт движок
queue-status-held-tooltip = Сейчас его некому взять: { $reason }. Начнётся само, как только станет можно.
queue-status-paused-tooltip = Переписывание на паузе; «Продолжить» на панели продолжает.
queue-status-rewriting = Переписывается…
queue-status-rewriting-chunk = Переписывается абзац { $chunk } из { $chunks }.
queue-status-rewriting-tooltip = Переписывается дежурным движком.
queue-status-working = Выполняется…
queue-status-rewritten = Переписано
queue-status-partly-rewritten = Переписано частично
queue-status-rewrite-failed = Переписать не удалось
queue-status-cancelled = Переписывание отменено
queue-status-findings = Найдены метки
queue-said-working = В работе.
queue-said-cancelled = Отменено; ничего не записано.
queue-said-rewrite-failed = Переписывание не завершилось ({ $reason }); поверх оригинала ничего не записано.
queue-said-recorded-failed = Не завершилось ({ $reason }).
queue-said-rewritten = Переписано моделью { $model }, { $chunks ->
        [one] { $chunks } абзац
        [few] { $chunks } абзаца
       *[other] { $chunks } абзацев
    }. Результат — самый изменённый вариант из прошедших все проверки, а не оценка, что он читается лучше.
queue-said-partly-rewritten = { $kept } из { $chunks ->
        [one] { $chunks } абзаца
       *[other] { $chunks } абзацев
    } остались очищенным оригиналом: ни один кандидат не прошёл проверки.
queue-said-recorded-refused = Не сделано ({ $reason }); ничего не тронуто.
queue-said-recorded-found = { -layer-a } нашёл { $findings } и оставил { $kept }.
queue-said-recorded = Так записано в журнале.
queue-went-caller = Возвращено тому, кто просил, и нигде не сохранено.
queue-went-row = Хранится в этом списке, пока строку не уберут: «Скопировать результат» есть в меню «Действия».
queue-origin-panel = Из панели
queue-origin-launch = Указано в командной строке приложения
queue-origin-cli = Из командной строки
queue-origin-agent = От агента
queue-price = Около { $calls } обращений к модели, до { $tokens } токенов ответа.
toolbar-rewrite-all = Переписать всё
toolbar-rewrite-all-tooltip = Переписать каждый ожидающий текст дежурным движком, по одному — сначала будет названа цена, и ничего не начнётся без вашего согласия. Недоступно, пока таких нет или никто не дежурит.
toolbar-pause = Пауза
toolbar-pause-tooltip = Поставить на паузу очередь переписываний — всех окон и всех агентов. Документ, который переписывается, вернётся в ожидание с уже готовыми абзацами.
toolbar-resume = Продолжить
toolbar-resume-tooltip = Продолжить очередь переписываний с того места, где она остановилась.
toolbar-clear-finished = Убрать завершённые
toolbar-clear-finished-tooltip = Убрать все завершённые строки из списка и из журнала. Уже записанные результаты остаются на месте.
rewrite-price-title = Переписать { $count ->
        [one] { $count } документ
        [few] { $count } документа
       *[other] { $count } документов
    }?
rewrite-price-calls = Около { $expected } обращений к модели, не больше { $worst }.
rewrite-price-tokens = До { $tokens } токенов ответа.
rewrite-price-time = Около { $minutes } мин при скорости, которую намерила последняя проверка.
rewrite-price-time-unknown = Сколько это займёт, неизвестно: запустите «Проверить» на странице «Движок», чтобы измерить скорость.
rewrite-price-here = Ничто не покидает этот компьютер.
rewrite-price-away = Каждый документ будет отправлен на { $host }.
rewrite-price-go = Переписать
rewrite-cancel = Отмена
rewrite-send-title = Отправить { $count ->
        [one] { $count } документ
        [few] { $count } документа
       *[other] { $count } документов
    } на { $host }?
rewrite-send-body = «Обрабатывать поступившее» установлено на переписывание, а дежурный движок не на этом компьютере: только что поступившее будет отправлено туда для переписывания.
rewrite-send-go = Отправить и переписать
status-rewriting = Переписывание { $current } из { $total } · абзац { $chunk } из { $chunks }
status-rewriting-starting = Переписывание { $current } из { $total }
status-rewrites-held = Переписывания ждут движок: { $reason }
status-rewrites-paused = Переписывание на паузе · ждут: { $count }
compare-rewritten-banner = Переписанный текст в том виде, в каком он доставлен, — самый изменённый вариант из прошедших все проверки. Правки здесь ничего не сохраняют, а закрытие ничего не записывает.
compare-rewritten-kept = { $kept } из { $chunks } абзацев остались очищенным оригиналом: ни один кандидат не прошёл проверки.
compare-reset-rewritten = Вернуть переписанный текст
compare-reset-rewritten-tooltip = Вернуть переписанный текст в том виде, в каком он доставлен, и забыть правки.
settings-arrival-title = Обрабатывать поступившее
settings-arrival-description = Что происходит с тем, что попало в главное окно. «Ничего» ждёт кнопки; «Очистить» очищает сразу; «Переписать» переписывает дежурным движком — а если этот движок не на этом компьютере, каждое поступление спрашивает, прежде чем что-то отправить.
settings-arrival-nothing = Ничего — ждать кнопки
settings-arrival-clean = Очистить
settings-arrival-rewrite = Переписать
settings-journal-keep-title = Хранить завершённые строки
settings-journal-keep-description = Сколько завершённая строка остаётся в списке главного окна — кто бы о ней ни просил, включая командную строку и агентов. Строка хранит, что произошло и куда ушёл результат, но никогда не текст. Уже записанные результаты этим не удаляются.
settings-journal-days = { $days ->
        [one] { $days } день
        [few] { $days } дня
       *[other] { $days } дней
    }

## E4-6b — the command line
cli-arg-record = Оставить строку об этом просмотре в журнале приложения, который показывает его главное окно. Просмотр ничего не меняет и без этого флага не записывается.
cli-arg-no-record = Не оставлять строку об этом запуске в журнале приложения. Без флага запуск виден в главном окне приложения как пришедший из командной строки.
cli-arg-out-rewrite = Файл вывода или `-` для стандартного вывода. По умолчанию `<name>.rewritten.<ext>` рядом с исходным — `<name>.cleaned.<ext>` очистки это другой результат, — а при чтении из стандартного ввода — стандартный вывод; запись поверх исходника требует явного флага и никогда не делается по умолчанию.
cli-journal-too-old = В журнал приложения ничего не записано: его база данных { $path } от более старой версии; приложение обновит её при следующем запуске.
cli-journal-newer = В журнал приложения ничего не записано: его база данных { $path } записана более новой версией { -brand-name }.
cli-journal-unwritable = В журнал приложения ничего не записано: в его базу данных { $path } не удалось записать ({ $reason }).

## E4-6b — the host verification's fixes
queue-action-remove-waited = Этого переписывания ждёт агент или командная строка. Сначала отмените его: вызвавшему сообщат, и строку можно будет убрать.
queue-said-rewrite-exists = По пути { $path } уже есть файл, а переписывание никогда не пишет поверх файла, который не оно создало. Ничего не переписано; «Заменить существующий результат» в меню «Действия» пишет поверх этого одного файла.
rewrite-consent-title = { $count ->
        [one] Отправить ждущий документ на { $host }?
        [few] Отправить { $count } ждущих документа на { $host }?
       *[other] Отправить { $count } ждущих документов на { $host }?
    }
rewrite-consent-body-here = Их попросили переписать, когда переписывание оставалось на этом компьютере. Сейчас назначен движок { $host }: каждый документ будет отправлен туда.
rewrite-consent-body-away = Их попросили переписать, когда переписывание шло на { $was }. Сейчас назначен движок { $host }: каждый документ будет отправлен туда.
rewrite-consent-hold = Ничего не начнётся, пока вы не ответите. «Отмена» оставляет их ждать; «Продолжить» на панели спрашивает снова, как и назначение другого движка.
rewrite-consent-go = Отправить
status-rewrites-asking = Переписывание ждёт вашего ответа: отправить на { $host }?
queue-status-asking-tooltip = Ждёт вашего ответа: назначенный движок отправил бы документ на { $host }, а просили не туда.
cli-rewritten-exists = { $path } уже существует, а переписывание никогда не пишет поверх файла, который не оно создало. Укажите другой файл через -o или замените сам входной файл через --in-place — он сначала откладывает оригинал. Ничего не переписано.

## E4-6c
##
## Раздел «Переписывание» окна настроек: шаблоны, которые получает модель,
## язык-посредник обратного перевода, проверка шаблона на встроенном образце
## и адаптация на другой язык моделью. $key, $rule, $tactic, $guard,
## $marker, $placeholder, $variable, $name и $suggestion — форматы, не
## переводятся. Названия языков приходят из `prompts-lang-*`.

settings-section-prompts = Переписывание
settings-prompts-title = Переписывание
settings-prompts-description = Как модель просят переписывать: шаблоны, которые она получает, на каждом языке, и язык, через который идёт обратный перевод.
settings-prompts-pivot-title = Обратный перевод через
settings-prompts-pivot-description = Язык, на который back_translate переводит абзац и с которого переводит обратно. Язык самого документа для этого документа означает выбор по умолчанию: перевод с английского на английский — не перевод.
prompts-pivot-by-document = По языку документа
prompts-pivot-unread = В строке языка-посредника записано { $value }, и эта версия не может это использовать: действует выбор по умолчанию, а строка остаётся как есть, пока её не заменит выбор здесь.
prompts-lang-en = английский
prompts-lang-ru = русский
prompts-lang-de = немецкий
prompts-turn-system = system
prompts-turn-user = user
prompts-banner-what = Шаблон — это то, что получает модель, чтобы переписать один абзац: ход system с правилами и ход user с задачей. Хранятся только ваши изменения; «Вернуть поставляемый» удаляет ваше.
prompts-banner-markers = Маркеры вокруг абзаца и его контекста и вставки ⟦n⟧ пишет сам продукт; вручную их писать нельзя.
prompts-banner-protected = { $variable } обязательна один раз на шаг, в любом из ходов: без правила о вставках каждый абзац с кодом или ссылкой бракуется.
prompts-banner-same-rows = Командная строка и агенты по MCP переписывают по этим же шаблонам. Файл --prompts командной строки или аргумент templates агента накладывает свои поверх на один запуск и ничего здесь не меняет.
prompts-banner-away = Переписывание отправляет собранный промпт вместе с документом на { $origin }.
prompts-variables-title = Переменные
prompts-var-text = Абзац между маркерами. Только в ходе user, ровно один раз в ходе user каждого шага.
prompts-var-prev-context = Конец предыдущего абзаца между своими маркерами и фраза «не переписывай его»; для первого абзаца — ничего. Только в ходе user, не больше одного раза. Без неё контекст не отправляется.
prompts-var-protected = Фраза о вставках ⟦n⟧; ничего, если в абзаце их нет. В любом ходе, хотя бы один раз на шаг.
prompts-var-intensity = Пункт интенсивности; для умеренной — ничего. Только в ходе user, необязательна: без неё интенсивность ни на что не влияет.
prompts-var-no-names = Переменной для названия языка нет: шаблон называет свой язык сам, своими словами.
prompts-slots-title = Шаблоны
prompts-tactic-structural-note = Используется только после подтверждения.
prompts-tactic-code-note = Не в этой версии: можно править, но пока ничем не используется.
prompts-tag-hand = ваш
prompts-tag-machine = адаптирован, не просмотрен
prompts-tag-machine-reviewed = адаптирован, просмотрен
prompts-tag-unreadable = нечитаемая строка
prompts-tag-stale = устарел
prompts-slot-heading = { $language } · { $tactic } · шаг { $step } · ход { $turn }
prompts-reading = Читаю шаблоны…
prompts-origin-shipped = Действует поставляемый шаблон.
prompts-origin-hand = Действует ваш шаблон, написанный вручную.
prompts-origin-hand-adapted = Действует ваш шаблон, адаптированный вручную с языка: { $source }.
prompts-origin-machine = Действует шаблон, адаптированный моделью с языка: { $source }, — не просмотрен. Сохраните его один раз, и он будет отмечен просмотренным.
prompts-origin-machine-reviewed = Действует шаблон, адаптированный моделью с языка: { $source }, — просмотрен.
prompts-unread = Эта версия не может прочитать строку, поэтому действует поставляемый шаблон. Строка остаётся как есть, пока «Вернуть поставляемый» её не удалит: { $value }
prompts-unread-not-json = Строка — не JSON, поэтому действует поставляемый шаблон. Строка остаётся как есть, пока «Вернуть поставляемый» её не удалит.
prompts-coverage-here = Это изменение действует только для документов на языке: { $here }; для документов на других языках ({ $others }) работает поставляемый шаблон.
prompts-coverage-elsewhere = Ваши изменения этого шаблона ({ $changed }) здесь не действуют: для документов на языке { $here } работает поставляемый.
prompts-stale-shipped = Поставляемый шаблон изменился после того, как ваш был от него сделан. Ваш по-прежнему действует, ничего не сливается.
prompts-keep-mine = Оставить мой
prompts-kept = Оставлен: ваш теперь отмечен как сделанный от сегодняшнего поставляемого шаблона.
prompts-stale-source = Исходник ({ $source }) изменился после того, как от него сделана эта адаптация ({ $target }). Адаптация по-прежнему действует.
prompts-stale-source-was-shipped = Она была сделана от поставляемого шаблона; ниже — что с тех пор изменилось в исходнике.
prompts-stale-source-unknown = Прежний исходник не хранится, поэтому показан только сегодняшний.
prompts-diff-legend-shipped = − только в сегодняшнем поставляемом · + только в вашем
prompts-diff-legend-before-after = − было · + стало
prompts-unsaved = Есть несохранённые правки. Если выбрать другой шаблон, они пропадут.
prompts-adapted-from-label = Адаптирован вручную с:
prompts-adapted-from-own = Написан на этом языке
prompts-save = Сохранить
prompts-undo-edits = Отменить правки
prompts-reset = Вернуть поставляемый
prompts-saved = Сохранено.
prompts-saved-warnings = Сохранено, с предупреждениями выше.
prompts-saved-reviewed = Сохранено и отмечено просмотренным.
prompts-unchanged = Сохранять нечего: это поставляемый шаблон.
prompts-refused = Не сохранено: шаблон нарушает правило выше. Ничего не записано.
prompts-reset-done = Возвращён поставляемый шаблон.
prompts-reset-refused = Не возвращено: ход { $turn } этого шага полагается на этот ход в части { $variable } ({ $rule }). Сначала верните тот ход или добавьте в него { $variable }.
prompts-write-failed = Не удалось записать строку: { $reason }
prompts-problem-error = Ошибка
prompts-problem-warning = Предупреждение
prompts-problem-at = (строка { $line }, столбец { $column })
prompts-problem-unknown-variable = { $name } — не переменная.
prompts-problem-unknown-variable-suggest = { $name } — не переменная; может быть, { $suggestion }?
prompts-problem-unclosed-open = У этой открывающей скобки нет закрывающей в той же строке. Буквальная скобка пишется дважды.
prompts-problem-unclosed-close = У этой закрывающей скобки нет открывающей в той же строке. Буквальная скобка пишется дважды.
prompts-problem-missing-text = Нет { $variable }: ход user содержит её ровно один раз.
prompts-problem-missing-protected = { $variable } нет ни в одном ходе этого шага; она должна быть в одном из них.
prompts-problem-missing-other = Нет { $variable }.
prompts-problem-repeated = { $variable } встречается { $count } раз(а); можно один раз.
prompts-problem-misplaced = { $variable } допустима только в ходе user, не в system.
prompts-problem-marker = { $marker } пишет продукт, а не шаблон.
prompts-problem-bracket = Скобки ⟦ и ⟧ — вставки документа; шаблон не может их писать.
prompts-problem-empty = Шаблон пуст. Чтобы работал поставляемый, верните поставляемый.
prompts-problem-too-long = Около { $tokens } токенов: больше десятой части окна модели ({ $limit } токенов), и абзацу останется слишком мало места.
prompts-problem-script = Большинство букв этого шаблона — не { $script }, письменность его набора: модель склонна отвечать на языке инструкции.
prompts-script-latin = латиница
prompts-script-cyrillic = кириллица
prompts-problem-nothing-but-text = В ходе user нет указания вокруг переменных, и неясно, что модель должна сделать.
prompts-problem-no-intensity = Интенсивность задана, а в шаблоне нет { $variable }, поэтому интенсивность ни на что не влияет.
prompts-problem-stale = Поставляемый шаблон изменился после того, как этот был от него сделан.
prompts-problem-variables-differ = Переменные адаптации не совпадают с исходником. Не хватает: { $missing }. Лишние: { $extra }.
prompts-none = нет
prompts-check = Проверить шаблон
prompts-stop = Остановить
prompts-check-note = Это проверка шаблона, а не переписывание документа: встроенный абзац-образец ({ $language }) переписывается по шаблону в том виде, в каком он сейчас в поле, — сохранён он или нет.
prompts-check-whole-tactic = Выполняются оба шага { $tactic }, поэтому вердикт выносится по тексту, который вернулся.
prompts-sent-to = Проверка отправляет образец и шаблоны, а адаптация — исходный шаблон на { $origin }. Больше ничего с этой машины не уходит.
prompts-check-errors = Шаблон с ошибками проверить нельзя.
prompts-check-code = code нет в этой версии, проверять его не на чем.
prompts-check-other-broken = Сохранённый шаблон { $key } нарушает правило, и шаг не соберётся. Сначала исправьте или верните его.
prompts-checking = Проверяю…
prompts-check-loading = Загружаю модель: { $percent } %
prompts-check-step = Шаг { $step }, язык { $language }: { $tokens } токенов за { $seconds } с. Ответ модели:
prompts-check-stripped = Зачистка убрала: { $what }
prompts-check-guard-passed = { $guard }: пройдено
prompts-check-guard-rejected = { $guard }: отбраковано. { $reason }
prompts-check-passed = Вердикт: это был бы кандидат. Дивергенция { $divergence }, длина { $ratio } от образца.
prompts-check-rejected = Вердикт: это было бы отбраковано. { $why }
prompts-check-time = Всего { $tokens } токенов за { $seconds } с.
prompts-check-cancelled = Проверка остановлена.
prompts-check-failed = Проверка не выполнилась: { $reason }
prompts-stripped-think = рассуждения модели
prompts-stripped-marker = { $marker }, { $count } раз(а)
prompts-stripped-fence = блок кода вокруг ответа
prompts-stripped-quotes = кавычки { $open } { $close } вокруг ответа
prompts-reason-placeholder-missing = { $placeholder } не вернулась.
prompts-reason-placeholder-duplicated = { $placeholder } вернулась { $count } раз(а).
prompts-reason-placeholder-invented = { $placeholder } не было в тексте.
prompts-reason-number-missing = Пропало число { $value }.
prompts-reason-length-drift = Длина ушла на { $ratio } от исходной, за пределы { $min }–{ $max }.
prompts-reason-script-drift = Доля букв письменности { $script } сдвинулась на { $points } п. п.
prompts-reason-identifier-missing = Пропало { $token }.
prompts-reason-item-broken = Пункт списка вернулся разбитым на строки.
prompts-failure-overflow = запросу нужно { $used } токенов из { $limit }
prompts-rejected-engine = Шаг { $step }: движок не справился. { $reason }
prompts-rejected-truncated = Шаг { $step } оборвался на пределе токенов.
prompts-rejected-empty = Шаг { $step } ничего не ответил.
prompts-rejected-guard = Отбраковал guard { $guard }: { $reason }
prompts-rejected-language = Это не { $expected }: текст читается как { $found }.
prompts-rejected-language-unknown = Это не { $expected }: текст не читается ни как один язык, известный этой версии.
prompts-rejected-restore = Его нельзя вернуть в документ: { $reason }
prompts-rejected-no-op = Это образец, если не считать пунктуации: дивергенция { $divergence }, меньше { $floor }.
prompts-rejected-marker = Ответ на шаг { $step } содержал { $marker }, поэтому следующий шаг задать нельзя.
prompts-adapt-from = Адаптировать моделью с языка: { $language }
prompts-adapt-note = Отправляет шаблон ({ $source }) — не документ — текущему исполнителю, и тот пишет версию на языке: { $target }. Она сохраняется, только если проходит те же правила, и отмечается непросмотренной.
prompts-adapt-own-template = Этот шаблон написан на своём языке; адаптация моделью заменила бы его. Чтобы адаптировать моделью, сначала верните поставляемый.
prompts-adapt-no-such-slot = у этого шаблона нет версии на том языке
prompts-adapting = Адаптирую…
prompts-adapt-saved = Адаптировано и сохранено, отмечено непросмотренным. Прочитайте и сохраните один раз, чтобы отметить просмотренным.
prompts-adapt-refused = Адаптация модели нарушает правило ниже, поэтому ничего не сохранено. Что она написала:
prompts-adapt-failed = Адаптация не выполнилась: { $reason }
prompts-adapt-truncated = Ответ модели оборвался, поэтому ничего не сохранено.
prompts-adapt-empty = Модель ничего не ответила, поэтому ничего не сохранено.
prompts-adapt-cancelled = Адаптация остановлена; ничего не сохранено.
prompts-shipped-show = Показать поставляемый шаблон
prompts-shipped-hide = Скрыть поставляемый шаблон
prompts-fragments-title = Фразы, которые продукт добавляет здесь. В этой версии их нельзя править.
prompts-fragment-protected = Во что превращается переменная о вставках, когда в абзаце они есть:
prompts-fragment-context = Что идёт после контекста:
prompts-fragment-light = Во что превращается переменная интенсивности при слабой:
prompts-fragment-strong = Во что превращается переменная интенсивности при сильной:
prompts-fragment-fallback = Что добавляется к английскому набору для документа, язык которого не распознан:
prompts-problem-invisible = { $character } — невидимый символ, который удаляет очистка, поэтому шаблон не может его содержать (всего: { $count }).
prompts-save-shipped-text = Это поставляемый шаблон, а хранятся только ваши изменения, поэтому ничего не записано. Чтобы действовал поставляемый шаблон, верните поставляемый.
prompts-save-unreadable = Эта версия не может прочитать сохранённую строку, а сохранение заменило бы её, поэтому ничего не записано. «Вернуть поставляемый» удаляет строку; затем сохраните свой.
prompts-save-while-adapting = Сохранение ждёт, пока модель адаптирует этот шаблон. Остановите адаптацию или дождитесь её.
prompts-adapt-unreadable = Эта версия не может прочитать сохранённую строку, а адаптация моделью заменила бы её. Сначала верните поставляемый.
prompts-adapt-overtaken = Шаблон изменился, пока модель его адаптировала, поэтому адаптация не сохранена поверх него.
prompts-adapt-not-stored = Ничего не сохранено. Что написала модель:
prompts-stale-source-keep = Сохранение не снимает это предупреждение. «Оставить мой» отмечает ваш как адаптированный от сегодняшнего шаблона ({ $source }).
prompts-kept-source = Оставлен: ваш теперь отмечен как адаптированный от сегодняшнего исходника.

const translations = {
        fr: {
            language: "Langue",
            tabAll: "Tous", tabMarkets: "Marchés", tabEnergy: "Énergie", tabRisks: "Veille", tabAIS: "AIS", languageAria: "Langue d’affichage", brandSubtitle: "Terminal marchés & énergie", clockCaption: "Heure locale · rafraîchissement auto", refresh: "↻ Actualiser",
            eyebrow: "Terminal de marché · données sourcées", pageTitle: "Marchés, énergie & risques",
            intro: "Prix affichés en EUR et CHF. Les cadences, sources et limites sont indiquées pour distinguer les cotations live des références quotidiennes.",
            marketsAria: "Marchés en direct", cryptoTitle: "Marchés crypto · cotations live", cryptoSubtitle: "Ticks Kraken et Binance · conversions indicatives en EUR et CHF",
            waitingBot: "En attente du bot", asset: "Actif", exchange: "Flux", currency: "Devise", chartAria: "Graphique de prix crypto live",
            chartWaiting: "Démarre le bot pour recevoir les ticks.", startBotPrices: "Démarre le bot pour charger les prix.",
            spread: "Écart", cryptoDisclaimer: "Prix du dernier échange public, pas un prix d’achat garanti. Conversion indicative depuis USDT≈USD avec les taux de référence quotidiens BCE. Le graphique conserve les ticks reçus depuis l’ouverture de la page.",
            fiatTitle: "Devises fiduciaires", fiatSubtitle: "Unités de devise pour 1 EUR", loading: "Chargement", loadingFx: "Chargement des taux de référence…",
            dailyReference: "Référence quotidienne", fxDisclaimer: "Taux de référence BCE via Frankfurter — ce ne sont pas des cotations Forex tick par tick.",
            ecbLink: "BCE · taux de change ↗", inboundFlows: "Flux entrants observés", outboundFlows: "Flux sortants observés", latestSource: "Dernière publication source",
            networkTitle: "Réseau gazier européen", networkSubtitle: "Points transfrontaliers et sens des observations publiées", loadingPoints: "Chargement des points ENTSOG…",
            mapAria: "Carte schématique des points de flux gazier ENTSOG", entry: "Entrée", exit: "Sortie",
            mapDisclaimer: "Schéma de positions publié par ENTSOG, pas une carte de routes de pipelines. Les données sont des observations de points, pas des totaux nationaux.",
            newsTitle: "Actualités & risques géopolitiques", newsWindow: "Actualités · dernières 24 h", loadingSources: "Chargement des sources…",
            newsDisclaimer: "Les articles sont des informations rapportées, non des incidents vérifiés ni des prédictions.",
            ttfTitle: "TTF · aucune cotation vérifiée", ttfSubtitle: "Prix du gaz européen — fournisseur de marché requis", unavailable: "Indisponible",
            ttfUnavailableTitle: "Prix et unité non affichés", ttfUnavailableText: "Aucune cotation TTF vérifiable n’est connectée. Le tableau de bord n’invente ni cours ni unité de cotation. Les spécifications ICE décrivent le contrat, elles ne fournissent pas un flux de prix.",
            ttfSource: "Référence du contrat uniquement :", iceLink: "ICE Dutch TTF ↗", flowsTitle: "Dernières observations de flux",
            flowsSubtitle: "Valeurs publiées au point · unité native de la source", entsogLink: "Source ENTSOG ↗",
            pointOperator: "Point / opérateur", direction: "Sens", publishedValue: "Valeur publiée", periodUpdated: "Période / mise à jour",
            waitingFlows: "En attente des données ENTSOG.", flowDisclaimer: "Les valeurs aux points ne sont ni dédoublonnées ni équivalentes aux importations nettes d’un pays.",
            cryptoMonitor: "Surveillance crypto", cryptoMonitorSubtitle: "Résumé du bot Kraken / Binance · panneau secondaire", readingLocal: "Lecture des observations locales…",
            footerDisclaimer: "Données gaz : ENTSOG Transparency Platform. Actualités : GDELT ou flux BBC World de repli, avec liens vers les éditeurs d’origine. Les flux peuvent être publiés avec retard et les actualités nécessitent une vérification indépendante.",
            waitingTicks: "En attente des ticks de marché…", waitingFx: "En attente des taux de conversion…", ticks: "ticks",
            utc: "UTC", pair: "Paire", grossSpread: "Spread brut", health: "Santé",
            allPricesFresh: "Tous les prix récents", partialPrices: "Prix récents partiels", startBot: "Démarre le bot", stalePrices: "Prix périmés",
            unavailableFeed: "Flux indisponible", fxAvailable: "Taux BCE disponibles", fxUnavailable: "Taux indisponibles",
            publishedOn: "Publié le {date}", reference: "EUR / {currency}", inverseReference: "CHF / EUR",
            inboundNote: "observations de points (pas un total national)", outboundNote: "observations de points (pas un total national)",
            recentPublication: "Publication récente reçue", oldPublication: "Dernière publication ancienne", noUsableObservation: "Aucune observation exploitable",
            recentSourceData: "Données source récentes", delayedSourceData: "Données retardées / absentes",
            noMapPoints: "Aucun point géographique exploitable dans les données récentes",
            staleFlowUnavailable: "La source ENTSOG ne fournit pas de valeur récente exploitable. Réessaie plus tard.",
            noFlowObservation: "Aucune observation de flux n’est actuellement disponible.", entryDirection: "↙ Entrée", exitDirection: "↗ Sortie",
            sourceUpdate: "Mise à jour source : {date}", officialEntsog: "Données officielles ENTSOG.",
            fxFailure: "Impossible de charger les taux de change. Le serveur réessaiera automatiquement.",
            flowFailure: "Impossible de charger les données ENTSOG. Le serveur réessaiera automatiquement.",
            noArticles: "Aucun article correspondant dans la fenêtre de recherche.", mediaSource: "Source média",
            newsFailure: "Impossible de charger les actualités. Le flux sera réessayé automatiquement.",
            noCryptoObservation: "Aucune observation crypto locale.", cryptoLogUnavailable: "Journal indisponible",
            botStatusFresh: "Données récentes", botStatusStale: "Données obsolètes", botStatusNone: "Aucune observation",
            chartNoPrice: "prix indisponible", chartStale: "dernier prix périmé", chartReceived: "Reçu", chartLastTickStale: "Périmé · dernier tick reçu",
            conversionUnavailable: "Conversion EUR/CHF indisponible", conversionUnavailableShort: "conversion indisponible",
            staleAge: "Périmé · {age}s", receivedAge: "Reçu il y a {age}s", staleAgeLabel: "Périmé · dernier tick reçu il y a {age}s",
            unknownError: "Erreur inattendue",
            period: "Période", day: "Jour", week: "Semaine", month: "Mois",
            historyLoading: "Historique local persistant", exportHistory: "Exporter les ticks CSV",
            historyReady: "{count} points agrégés · source USDT", historyEmpty: "Aucun historique enregistré pour cette période.",
            priceAlertsTitle: "Alertes de prix EUR / CHF", above: "Au-dessus", below: "En-dessous",
            priceThreshold: "Seuil de prix", addAlert: "Ajouter l’alerte", noAlerts: "Aucune alerte configurée.",
            alertNeedsPage: "Les alertes sont évaluées avec les cotations récentes lorsque le dashboard est ouvert.",
            priceAlertsUnavailable: "Création d’alertes de seuil indisponible dans cette version Rust. Les alertes de simulation restent visibles.",
            alertTriggered: "{symbol} a franchi le seuil de {target} {currency}.",
            alertFailure: "Impossible d’enregistrer ou d’évaluer les alertes.",
            sourceStatusTitle: "État des sources", sourceStatusSubtitle: "Disponibilité et dernier événement observé",
            sourceRetrieved: "Dernier événement : {date}", live: "Live", delayed: "Retardé", stale: "Périmé",
            sourceUnavailable: "Indisponible", dailyReportHint: "Rapport journalier : cours observés, flux ENTSOG et titres rapportés.",
            downloadDailyReport: "Télécharger le rapport CSV du jour", fxAge: "Cours BCE du {date} · âge {age} j · source {source}",
            deleteAlert: "Supprimer l’alerte",
            aisTitle: "Suivi maritime AIS",
            aisSubtitle: "Proxy non-commercial · couverture terrestre uniquement",
            aisDisclaimer: "Données AIS via aisstream.io proxy non-commercial. Pas de redistribution, pas de données satellite, quota API. Marqué non-commercial proxy et non vérifié.",
            aisLoading: "Chargement des positions AIS…",
            aisGeojson: "Télécharger GeoJSON",
            aisUnavailable: "AIS indisponible"
        },
        ru: {
            language: "Язык",
            tabAll: "Все", tabMarkets: "Рынки", tabEnergy: "Энергия", tabRisks: "Наблюдение", tabAIS: "AIS", languageAria: "Язык интерфейса", brandSubtitle: "Терминал рынков и энергетики", clockCaption: "Местное время · автообновление", refresh: "↻ Обновить",
            eyebrow: "Рыночный терминал · данные с источниками", pageTitle: "Рынки, энергетика и риски",
            intro: "Цены показаны в EUR и CHF. Указаны периодичность, источники и ограничения, чтобы отличать текущие котировки от ежедневных справочных курсов.",
            marketsAria: "Рынки в реальном времени", cryptoTitle: "Крипторынки · текущие котировки", cryptoSubtitle: "Тики Kraken и Binance · ориентировочная конвертация в EUR и CHF",
            waitingBot: "Ожидание запуска бота", asset: "Актив", exchange: "Биржа", currency: "Валюта", chartAria: "График цены криптовалюты в реальном времени",
            chartWaiting: "Запустите бота, чтобы получать тики.", startBotPrices: "Запустите бота, чтобы загрузить цены.",
            spread: "Спред", cryptoDisclaimer: "Цена последней публичной сделки, а не гарантированная цена покупки. Ориентировочная конвертация из USDT≈USD использует ежедневные справочные курсы ЕЦБ. На графике сохраняются тики с момента открытия страницы.",
            fiatTitle: "Фиатные валюты", fiatSubtitle: "Количество единиц валюты за 1 EUR", loading: "Загрузка", loadingFx: "Загрузка справочных курсов…",
            dailyReference: "Ежедневный справочный курс", fxDisclaimer: "Справочные курсы ЕЦБ через Frankfurter — это не потоковые Forex-котировки.",
            ecbLink: "ЕЦБ · валютные курсы ↗", inboundFlows: "Наблюдаемые входящие потоки", outboundFlows: "Наблюдаемые исходящие потоки", latestSource: "Последняя публикация источника",
            networkTitle: "Европейская газовая сеть", networkSubtitle: "Трансграничные точки и направления опубликованных наблюдений", loadingPoints: "Загрузка точек ENTSOG…",
            mapAria: "Схематическая карта точек газовых потоков ENTSOG", entry: "Вход", exit: "Выход",
            mapDisclaimer: "Схема позиций ENTSOG, а не карта маршрутов газопроводов. Это наблюдения в точках, а не национальные итоги.",
            newsTitle: "Новости и геополитические риски", newsWindow: "Новости · последние 24 ч", loadingSources: "Загрузка источников…",
            newsDisclaimer: "Это сообщения СМИ, а не проверенные инциденты или прогнозы.",
            ttfTitle: "TTF · проверенная котировка отсутствует", ttfSubtitle: "Цена европейского газа — требуется поставщик рыночных данных", unavailable: "Недоступно",
            ttfUnavailableTitle: "Цена и единица не указаны", ttfUnavailableText: "Проверенный источник котировок TTF не подключён. Панель не выдумывает цену или единицу котировки. Спецификация ICE описывает контракт, но не предоставляет поток цен.",
            ttfSource: "Только спецификация контракта:", iceLink: "ICE Dutch TTF ↗", flowsTitle: "Последние наблюдения потоков",
            flowsSubtitle: "Опубликованные значения в точке · исходная единица", entsogLink: "Источник ENTSOG ↗",
            pointOperator: "Точка / оператор", direction: "Направление", publishedValue: "Опубликованное значение", periodUpdated: "Период / обновление",
            waitingFlows: "Ожидание данных ENTSOG.", flowDisclaimer: "Значения в точках не дедуплицированы и не равны чистому национальному импорту.",
            cryptoMonitor: "Мониторинг криптовалют", cryptoMonitorSubtitle: "Сводка бота Kraken / Binance · дополнительная панель", readingLocal: "Чтение локальных наблюдений…",
            footerDisclaimer: "Газовые данные: платформа прозрачности ENTSOG. Новости: GDELT или резервный RSS BBC World со ссылками на издателей. Данные могут публиковаться с задержкой; новости требуют независимой проверки.",
            waitingTicks: "Ожидание рыночных тиков…", waitingFx: "Ожидание курсов конвертации…", ticks: "тиков",
            utc: "UTC", pair: "Пара", grossSpread: "Валовый спред", health: "Состояние",
            allPricesFresh: "Все цены актуальны", partialPrices: "Часть цен актуальна", startBot: "Запустите бота", stalePrices: "Цены устарели",
            unavailableFeed: "Поток недоступен", fxAvailable: "Доступны курсы ЕЦБ", fxUnavailable: "Курсы недоступны",
            publishedOn: "Опубликовано: {date}", reference: "EUR / {currency}", inverseReference: "CHF / EUR",
            inboundNote: "наблюдения в точках (не национальный итог)", outboundNote: "наблюдения в точках (не национальный итог)",
            recentPublication: "Получена свежая публикация", oldPublication: "Последняя публикация устарела", noUsableObservation: "Нет доступных наблюдений",
            recentSourceData: "Свежие данные источника", delayedSourceData: "Данные задержаны / отсутствуют",
            noMapPoints: "В недавних данных нет доступных географических точек",
            staleFlowUnavailable: "В ENTSOG нет доступных свежих значений. Попробуйте позже.",
            noFlowObservation: "Сейчас нет доступных наблюдений потоков.", entryDirection: "↙ Вход", exitDirection: "↗ Выход",
            sourceUpdate: "Обновление источника: {date}", officialEntsog: "Официальные данные ENTSOG.",
            fxFailure: "Не удалось загрузить валютные курсы. Сервер повторит попытку автоматически.",
            flowFailure: "Не удалось загрузить данные ENTSOG. Сервер повторит попытку автоматически.",
            noArticles: "За выбранный период подходящих статей нет.", mediaSource: "Источник СМИ",
            newsFailure: "Не удалось загрузить новости. Будет выполнена повторная попытка.",
            noCryptoObservation: "Нет локальных наблюдений криптовалют.", cryptoLogUnavailable: "Журнал недоступен",
            botStatusFresh: "Свежие данные", botStatusStale: "Данные устарели", botStatusNone: "Нет наблюдений",
            chartNoPrice: "цена недоступна", chartStale: "последняя цена устарела", chartReceived: "Получено", chartLastTickStale: "Устарело · последний тик",
            conversionUnavailable: "Конвертация EUR/CHF недоступна", conversionUnavailableShort: "конвертация недоступна",
            staleAge: "Устарело · {age}с", receivedAge: "{age}с назад", staleAgeLabel: "Устарело · последний тик {age}с назад",
            unknownError: "Непредвиденная ошибка",
            period: "Период", day: "День", week: "Неделя", month: "Месяц",
            historyLoading: "Сохраняемая локальная история", exportHistory: "Экспорт тиков CSV",
            historyReady: "{count} агрегированных точек · источник USDT", historyEmpty: "За этот период сохранённых данных нет.",
            priceAlertsTitle: "Ценовые оповещения EUR / CHF", above: "Выше", below: "Ниже",
            priceThreshold: "Ценовой порог", addAlert: "Добавить оповещение", noAlerts: "Оповещений пока нет.",
            alertNeedsPage: "Оповещения проверяются по свежим котировкам, пока панель открыта.",
            priceAlertsUnavailable: "Создание пороговых оповещений пока недоступно в версии Rust. События симуляции видны ниже.",
            alertTriggered: "{symbol}: достигнут порог {target} {currency}.",
            alertFailure: "Не удалось сохранить или проверить оповещения.",
            sourceStatusTitle: "Состояние источников", sourceStatusSubtitle: "Доступность и время последнего события",
            sourceRetrieved: "Последнее событие: {date}", live: "В реальном времени", delayed: "Задержка", stale: "Устарело",
            sourceUnavailable: "Недоступно", dailyReportHint: "Дневной отчёт: котировки, потоки ENTSOG и сообщения СМИ.",
            downloadDailyReport: "Скачать дневной CSV-отчёт", fxAge: "Курс ЕЦБ на {date} · возраст {age} дн. · источник {source}",
            deleteAlert: "Удалить оповещение",
            aisTitle: "Морской мониторинг AIS",
            aisSubtitle: "Некоммерческий прокси · только наземное покрытие",
            aisDisclaimer: "Данные AIS через aisstream.io некоммерческий прокси. Без перераспространения, без спутниковых данных, лимиты API. Помечено как non-commercial proxy и непроверено.",
            aisLoading: "Загрузка позиций AIS…",
            aisGeojson: "Скачать GeoJSON",
            aisUnavailable: "AIS недоступно"
        },
        en: {
            language: "Language",
            tabAll: "All", tabMarkets: "Markets", tabEnergy: "Energy", tabRisks: "Monitoring", tabAIS: "AIS", languageAria: "Display language", clockCaption: "Local time · auto-refresh", refresh: "↻ Refresh",
            brandSubtitle: "Markets & energy terminal",
            eyebrow: "Market terminal · sourced data", pageTitle: "Markets, energy & risk",
            intro: "Prices are shown in EUR and CHF. Cadence, sources, and limitations distinguish live quotes from daily reference rates.",
            marketsAria: "Live markets", cryptoTitle: "Crypto markets · live quotes", cryptoSubtitle: "Kraken and Binance ticks · indicative EUR and CHF conversions",
            waitingBot: "Waiting for bot", asset: "Asset", exchange: "Exchange", currency: "Currency", chartAria: "Live crypto price chart",
            chartWaiting: "Start the bot to receive market ticks.", startBotPrices: "Start the bot to load prices.",
            spread: "Spread", cryptoDisclaimer: "Public last-trade price, not a guaranteed purchase price. Indicative conversion assumes USDT≈USD and uses daily ECB reference rates. The chart retains ticks received since this page was opened.",
            fiatTitle: "Fiat currencies", fiatSubtitle: "Currency units per 1 EUR", loading: "Loading", loadingFx: "Loading reference rates…",
            dailyReference: "Daily reference", fxDisclaimer: "ECB reference rates via Frankfurter — these are not tick-by-tick Forex quotes.",
            ecbLink: "ECB · exchange rates ↗", inboundFlows: "Observed inbound flows", outboundFlows: "Observed outbound flows", latestSource: "Latest source publication",
            networkTitle: "European gas network", networkSubtitle: "Cross-border points and direction of published observations", loadingPoints: "Loading ENTSOG points…",
            mapAria: "Schematic map of ENTSOG gas-flow points", entry: "Entry", exit: "Exit",
            mapDisclaimer: "Schematic of ENTSOG point locations, not a pipeline-route map. These are point observations, not national totals.",
            newsTitle: "News & geopolitical risks", newsWindow: "News · last 24 hours", loadingSources: "Loading sources…",
            newsDisclaimer: "Articles are reported information, not verified incidents or predictions.",
            ttfTitle: "TTF · no verified quote", ttfSubtitle: "European gas price — market-data provider required", unavailable: "Unavailable",
            ttfUnavailableTitle: "Price and unit not shown", ttfUnavailableText: "No verifiable TTF quote feed is connected. The dashboard does not invent a price or quote unit. ICE specifications describe the contract; they do not provide a price feed.",
            ttfSource: "Contract reference only:", iceLink: "ICE Dutch TTF ↗", flowsTitle: "Latest flow observations",
            flowsSubtitle: "Values published at the point · source-native unit", entsogLink: "ENTSOG source ↗",
            pointOperator: "Point / operator", direction: "Direction", publishedValue: "Published value", periodUpdated: "Period / update",
            waitingFlows: "Waiting for ENTSOG data.", flowDisclaimer: "Point values are not deduplicated and are not equivalent to a country's net imports.",
            cryptoMonitor: "Crypto monitoring", cryptoMonitorSubtitle: "Kraken / Binance bot summary · secondary panel", readingLocal: "Reading local observations…",
            footerDisclaimer: "Gas data: ENTSOG Transparency Platform. News: GDELT or BBC World RSS fallback, linked to original publishers. Source data may be delayed and news requires independent verification.",
            waitingTicks: "Waiting for market ticks…", waitingFx: "Waiting for conversion rates…", ticks: "ticks",
            utc: "UTC", pair: "Pair", grossSpread: "Gross spread", health: "Status",
            allPricesFresh: "All prices up to date", partialPrices: "Some prices up to date", startBot: "Start the bot", stalePrices: "Prices stale",
            unavailableFeed: "Feed unavailable", fxAvailable: "ECB rates available", fxUnavailable: "Rates unavailable",
            publishedOn: "Published {date}", reference: "EUR / {currency}", inverseReference: "CHF / EUR",
            inboundNote: "point observations (not a national total)", outboundNote: "point observations (not a national total)",
            recentPublication: "Recent publication received", oldPublication: "Latest publication is old", noUsableObservation: "No usable observations",
            recentSourceData: "Recent source data", delayedSourceData: "Data delayed / unavailable",
            noMapPoints: "No usable geographic points in recent data",
            staleFlowUnavailable: "ENTSOG has no recent usable value. Please try again later.",
            noFlowObservation: "No flow observations are currently available.", entryDirection: "↙ Entry", exitDirection: "↗ Exit",
            sourceUpdate: "Source update: {date}", officialEntsog: "Official ENTSOG data.",
            fxFailure: "Could not load exchange rates. The server will retry automatically.",
            flowFailure: "Could not load ENTSOG data. The server will retry automatically.",
            noArticles: "No matching articles in the search window.", mediaSource: "Media source",
            newsFailure: "Could not load news. The feed will be retried automatically.",
            noCryptoObservation: "No local crypto observations.", cryptoLogUnavailable: "Log unavailable",
            botStatusFresh: "Recent data", botStatusStale: "Data is stale", botStatusNone: "No observations",
            chartNoPrice: "price unavailable", chartStale: "last price is stale", chartReceived: "Received", chartLastTickStale: "Stale · last tick",
            conversionUnavailable: "EUR/CHF conversion unavailable", conversionUnavailableShort: "conversion unavailable",
            staleAge: "Stale · {age}s", receivedAge: "{age}s ago", staleAgeLabel: "Stale · last tick {age}s ago",
            unknownError: "Unexpected error",
            period: "Period", day: "Day", week: "Week", month: "Month",
            historyLoading: "Persistent local history", exportHistory: "Export ticks as CSV",
            historyReady: "{count} aggregated points · source USDT", historyEmpty: "No stored history for this period.",
            priceAlertsTitle: "EUR / CHF price alerts", above: "Above", below: "Below",
            priceThreshold: "Price threshold", addAlert: "Add alert", noAlerts: "No alerts configured.",
            alertNeedsPage: "Alerts are evaluated against fresh quotes while the dashboard is open.",
            priceAlertsUnavailable: "Price threshold alerts are not yet implemented in Rust. Paper simulation alerts remain visible below.",
            alertTriggered: "{symbol} crossed the {target} {currency} threshold.",
            alertFailure: "Could not save or evaluate alerts.",
            sourceStatusTitle: "Source status", sourceStatusSubtitle: "Availability and latest observed event",
            sourceRetrieved: "Latest event: {date}", live: "Live", delayed: "Delayed", stale: "Stale",
            sourceUnavailable: "Unavailable", dailyReportHint: "Daily report: observed prices, ENTSOG flows, and reported headlines.",
            downloadDailyReport: "Download today’s CSV report", fxAge: "ECB rate for {date} · age {age} days · source {source}",
            deleteAlert: "Delete alert",
            aisTitle: "Maritime AIS tracking",
            aisSubtitle: "Non-commercial proxy · terrestrial coverage only",
            aisDisclaimer: "AIS data via aisstream.io non-commercial proxy. No redistribution, no satellite data, API quotas apply. Marked non-commercial proxy and unverified.",
            aisLoading: "Loading AIS positions…",
            aisGeojson: "Download GeoJSON",
            aisUnavailable: "AIS unavailable"
        }
    };
    const supportedLanguages = Object.keys(translations);
    const languageLocales = { fr: "fr-FR", ru: "ru-RU", en: "en-GB" };
    let currentLanguage = "fr";
    try {
        const savedLanguage = localStorage.getItem("pravda-dashboard-language");
        if (supportedLanguages.includes(savedLanguage)) currentLanguage = savedLanguage;
    } catch {
        currentLanguage = "fr";
    }

    function t(key, values = {}) {
        const template = translations[currentLanguage][key] || translations.fr[key] || key;
        return template.replace(/\{(\w+)\}/g, (_, name) => safeText(values[name]));
    }

    function applyLanguage() {
        document.documentElement.lang = currentLanguage;
        document.title = {
            fr: "Pravda | Terminal marchés & énergie",
            ru: "Pravda | Рынки и энергетика",
            en: "Pravda | Markets & Energy"
        }[currentLanguage];
        for (const element of document.querySelectorAll("[data-i18n]")) {
            const key = element.dataset.i18n;
            element.textContent = t(key);
        }
        for (const element of document.querySelectorAll("[data-i18n-aria]")) {
            element.setAttribute("aria-label", t(element.dataset.i18nAria));
        }
        $("language-select").value = currentLanguage;
        updateClock();
        if (latestMarket) renderMarket(latestMarket);
        if (latestFx) renderFx(latestFx);
        refreshAlerts();
        refreshSources();
        $("alert-notice").textContent = t("priceAlertsUnavailable");
    }

    const $ = (id) => document.getElementById(id);
    const safeText = (value) => value == null ? "" : String(value);
    const setStatus = (element, state, label) => {
        element.className = `status ${state}`;
        element.replaceChildren();
        const dot = document.createElement("i");
        dot.className = "dot";
        element.append(dot, document.createTextNode(label));
    };
    const formatDate = (value) => {
        if (!value) return "—";
        const date = new Date(value);
        return Number.isNaN(date.getTime()) ? safeText(value) : date.toLocaleString(languageLocales[currentLanguage]);
    };
    const numericFormat = (value) => new Intl.NumberFormat(languageLocales[currentLanguage], { maximumFractionDigits: 2 }).format(value);
    const formatCurrency = (value, currency) => new Intl.NumberFormat(languageLocales[currentLanguage], {
        style: "currency",
        currency,
        maximumFractionDigits: 2
    }).format(value);
    function convertQuote(price, currency) {
        const usdPerEur = latestFx?.rates?.USD;
        const chfPerEur = latestFx?.rates?.CHF;
        if (!Number.isFinite(price) || price <= 0) return null;
        // Spot markets are USDT pairs: display the direct quote without a fiat feed.
        // No synthetic USD/EUR or USD/CHF rate is inferred when FX is unavailable.
        if (currency === "USDT") return price;
        if (!Number.isFinite(usdPerEur) || usdPerEur <= 0) return null;
        const eur = price / usdPerEur;
        if (currency === "EUR") return eur;
        return Number.isFinite(chfPerEur) && chfPerEur > 0 ? eur * chfPerEur : null;
    }
    const formatMarketValue = (price, currency) => currency === "USDT"
        ? `${numericFormat(price)} USDT`
        : formatCurrency(price, currency);
    const parsePrice = (value) => {
        const parsed = Number(String(value ?? "").trim().replace(",", "."));
        return Number.isFinite(parsed) && parsed > 0 ? parsed : null;
    };
    let historicalPoints = [];
    let latestMarket = null;
    let latestFx = null;

    function drawPriceChart() {
        const canvas = $("price-chart");
        const rect = canvas.getBoundingClientRect();
        const ratio = window.devicePixelRatio || 1;
        canvas.width = Math.max(1, Math.floor(rect.width * ratio));
        canvas.height = Math.max(1, Math.floor(rect.height * ratio));
        const context = canvas.getContext("2d");
        context.scale(ratio, ratio);
        const width = rect.width, height = rect.height;
        context.clearRect(0, 0, width, height);
        const symbol = $("chart-symbol").value;
        const exchange = $("chart-exchange").value;
        const currency = $("chart-currency").value;
        const rangeMs = { day: 86400000, week: 604800000, month: 2592000000 }[$("chart-period").value];
        const windowStart = Date.now() - rangeMs;
        const convertedHistory = historicalPoints.map(point => ({
            ...point,
            time: new Date(point.timestamp).getTime(),
            convertedPrice: convertQuote(point.price, currency)
        })).filter(point => Number.isFinite(point.convertedPrice));
        if (!convertedHistory.length) {
            context.fillStyle = getComputedStyle(document.documentElement).getPropertyValue("--muted").trim();
            context.font = "13px Segoe UI";
            context.fillText(latestFx ? t("historyEmpty") : t("waitingFx"), 18, height / 2);
            return;
        }
        const prices = convertedHistory.map(point => point.convertedPrice);
        const min = Math.min(...prices), max = Math.max(...prices);
        const padding = 20, range = max - min || Math.max(Math.abs(max) * 0.0001, 0.000001);
        context.strokeStyle = "rgba(158,172,194,.18)";
        context.lineWidth = 1;
        for (let i = 0; i < 4; i++) {
            const y = padding + i * (height - padding * 2) / 3;
            context.beginPath(); context.moveTo(padding, y); context.lineTo(width - padding, y); context.stroke();
        }
        context.beginPath();
        convertedHistory.forEach((point, index) => {
            const x = padding + Math.max(0, Math.min(1, (point.time - windowStart) / rangeMs)) * (width - padding * 2);
            const y = height - padding - (point.convertedPrice - min) / range * (height - padding * 2);
            if (index === 0) context.moveTo(x, y); else context.lineTo(x, y);
        });
        context.strokeStyle = getComputedStyle(document.documentElement).getPropertyValue("--cyan").trim(); context.lineWidth = 2; context.stroke();
        if (convertedHistory.length === 1) {
            const point = convertedHistory[0];
            const x = padding + Math.max(0, Math.min(1, (point.time - windowStart) / rangeMs)) * (width - padding * 2);
            const y = height - padding - (point.convertedPrice - min) / range * (height - padding * 2);
            context.beginPath(); context.arc(x, y, 3, 0, Math.PI * 2);
            context.fillStyle = getComputedStyle(document.documentElement).getPropertyValue("--cyan").trim(); context.fill();
        }
        const first = prices[0], last = prices[prices.length - 1];
        const change = first ? ((last - first) / first * 100) : 0;
        context.fillStyle = change >= 0
            ? getComputedStyle(document.documentElement).getPropertyValue("--green").trim()
            : getComputedStyle(document.documentElement).getPropertyValue("--red").trim();
        context.font = "12px Segoe UI";
        const tickCount = convertedHistory.reduce((total, point) => total + (point.ticks || 1), 0);
        context.fillText(`${change >= 0 ? "+" : ""}${change.toFixed(3)}% · ${tickCount} ${t("ticks")}`, padding, 16);
        context.fillStyle = getComputedStyle(document.documentElement).getPropertyValue("--muted").trim(); context.textAlign = "right";
        context.fillText(formatMarketValue(max, currency), width - padding, 16);
        context.fillText(formatMarketValue(min, currency), width - padding, height - 5);
        context.textAlign = "left";
        const dateOptions = { hour: "2-digit", minute: "2-digit" };
        if ($("chart-period").value !== "day") dateOptions.month = "short";
        context.fillText(new Date(windowStart).toLocaleString(languageLocales[currentLanguage], dateOptions), padding, height - 5);
        context.textAlign = "right";
        context.fillText(new Date().toLocaleString(languageLocales[currentLanguage], dateOptions), width - padding, height - 5);
        context.textAlign = "left";
    }

    // The compact watchlist mirrors the exact observed market snapshot used
    // for the central chart. No independent feed and never fabricated prices.
    function clearMarketSnapshot() {
        // A failed request or an authentication transition must also clear
        // the cached payload: applyLanguage() can otherwise replay its old
        // freshness flags and make an unavailable quote look recent again.
        latestMarket = null;
        document.querySelectorAll(".watchlist-row").forEach(button => {
            button.dataset.freshness = "unavailable";
            button.querySelector(".watch-price").textContent = "—";
            button.querySelector(".watch-freshness").textContent = "Indisponible";
        });
        $("crypto-quotes").replaceChildren();
        $("chart-price").textContent = "—";
        $("chart-age").textContent = t("unavailableFeed");
        setStatus($("market-feed-status"), "error", t("unavailableFeed"));
    }
    function clearMarketSession() {
        clearMarketSnapshot();
        latestFx = null;
        historicalPoints = [];
        $("fx-rates").textContent = t("fxFailure");
        $("fx-asof").textContent = "—";
        $("history-status").textContent = t("unavailableFeed");
        setStatus($("fx-status"), "error", t("fxUnavailable"));
        drawPriceChart();
    }
    window.pravdaClearMarketSession = clearMarketSession;
    function updateTerminalWatchlist(rows) {
        for (const row of rows) {
            const button = $("terminal-watchlist")
                .querySelector('[data-watch-symbol="' + row.symbol + '"]');
            if (!button) continue;
            const live = [row.binance, row.kraken].find(
                quote => quote?.status === "live" && Number.isFinite(quote.price));
            const observed = live || [row.binance, row.kraken].find(
                quote => Number.isFinite(quote?.price));
            const state = live ? "live" : observed ? "stale" : "unavailable";
            button.dataset.freshness = state;
            button.querySelector(".watch-price").textContent =
                observed ? formatMarketValue(observed.price, "USDT") : "—";
            button.querySelector(".watch-freshness").textContent =
                state === "live" ? "Récent" :
                state === "stale" ? "Périmé" : "Indisponible";
        }
    }
    document.querySelectorAll(".watchlist-row").forEach(button => {
        button.addEventListener("click", () => {
            const symbol = button.dataset.watchSymbol;
            if (!["BTC/USDT", "ETH/USDT", "SOL/USDT"].includes(symbol)) return;
            $("chart-symbol").value = symbol;
            $("chart-symbol").dispatchEvent(new Event("change", {bubbles: true}));
            document.querySelector('.tab-btn[data-tab="markets"]')?.click();
        });
    });

    function renderMarket(payload) {
        latestMarket = payload;
        if (typeof checkLocalAlerts === "function") checkLocalAlerts();
        const symbols = payload.symbols || {};
        const rows = [];
        for (const symbol of ["BTC/USDT", "ETH/USDT", "SOL/USDT"]) {
            const quotes = symbols[symbol] || {};
            const binance = quotes.Binance, kraken = quotes.Kraken;
            let spread = "—";
            if (binance?.status === "live" && kraken?.status === "live") {
                spread = `${(((kraken.price - binance.price) / ((kraken.price + binance.price) / 2)) * 100).toFixed(3)}%`;
            }
            rows.push({ symbol, binance, kraken, spread });
        }
        const allQuotes = Object.values(symbols).flatMap(quotes => Object.values(quotes));
        const liveQuoteCount = allQuotes.filter(quote => quote.status === "live").length;
        if (liveQuoteCount > 0) {
            const label = liveQuoteCount === allQuotes.length ? t("allPricesFresh") : t("partialPrices");
            setStatus($("market-feed-status"), "live", label);
        } else {
            setStatus($("market-feed-status"), "stale", payload.status === "waiting" ? t("startBot") : t("stalePrices"));
        }
        updateTerminalWatchlist(rows);
        const tbody = $("crypto-quotes");
        tbody.replaceChildren();
        for (const row of rows) {
            const tr = document.createElement("tr");
            const sym = document.createElement("td"); sym.className = "quote-symbol"; sym.textContent = row.symbol;
            const quoteCell = quote => {
                const cell = document.createElement("td");
                if (Number.isFinite(quote?.price)) {
                    const values = document.createElement("div");
                    values.className = "quote-values";
                    const eur = convertQuote(quote.price, "EUR");
                    const chf = convertQuote(quote.price, "CHF");
                    const directValue = document.createElement("span");
                    directValue.textContent = formatMarketValue(quote.price, "USDT");
                    const euroValue = document.createElement("span");
                    euroValue.textContent = eur === null ? "EUR —" : formatCurrency(eur, "EUR");
                    const francValue = document.createElement("span");
                    francValue.textContent = chf === null ? "CHF —" : formatCurrency(chf, "CHF");
                    values.append(directValue, euroValue, francValue);
                    cell.append(values);
                    if (quote.status !== "live") {
                        cell.className = "quote-stale";
                        cell.title = `Dernier tick reçu il y a ${quote.age_seconds}s`;
                        const stale = document.createElement("small");
                        stale.textContent = t("staleAge", { age: quote.age_seconds });
                        values.append(stale);
                    }
                } else {
                    cell.textContent = "—";
                }
                return cell;
            };
            const b = quoteCell(row.binance);
            const k = quoteCell(row.kraken);
            const s = document.createElement("td"); s.textContent = row.spread;
            if (row.spread !== "—") s.className = parseFloat(row.spread) >= 0 ? "positive" : "negative";
            tr.append(sym, b, k, s); tbody.append(tr);
        }
        const symbol = $("chart-symbol").value, exchange = $("chart-exchange").value;
        const currency = $("chart-currency").value;
        const quote = symbols[symbol]?.[exchange];
        const hasPrice = Number.isFinite(quote?.price);
        const euroPrice = hasPrice ? convertQuote(quote.price, "EUR") : null;
        const chfPrice = hasPrice ? convertQuote(quote.price, "CHF") : null;
        const selectedPrice = hasPrice ? convertQuote(quote.price, currency) : null;
        const dualCurrencyPrice = hasPrice && euroPrice !== null && chfPrice !== null
            ? `${formatCurrency(euroPrice, "EUR")} · ${formatCurrency(chfPrice, "CHF")}`
            : t("conversionUnavailable");
        const directPrice = hasPrice ? formatMarketValue(quote.price, "USDT") : "";
        $("chart-price").textContent = hasPrice
            ? `${symbol} · ${exchange} · ${directPrice} · ${dualCurrencyPrice}${quote.status === "live" ? "" : ` · ${t("chartStale")}`}`
            : `${symbol} · ${exchange} · ${t("chartNoPrice")}`;
        $("price-chart").setAttribute("aria-label", `${t("chartAria")} · ${currency}`);
        $("price-chart").dataset.selectedCurrency = currency;
        if (selectedPrice === null && hasPrice) $("chart-price").textContent += ` · ${t("conversionUnavailableShort")}`;
        $("chart-age").textContent = quote
            ? `${quote.status === "live" ? t("receivedAge", { age: quote.age_seconds }) : t("staleAgeLabel", { age: quote.age_seconds })} · ${formatDate(quote.timestamp)}`
            : t("chartWaiting");
        drawPriceChart();
    }

    function renderFx(payload) {
        latestFx = payload;
        if (typeof checkLocalAlerts === "function") checkLocalAlerts();
        const fxAvailable = payload.status !== "unavailable" && Object.keys(payload.rates || {}).length > 0;
        setStatus($("fx-status"), !fxAvailable ? "error" : payload.freshness === "stale" ? "stale" : "pending",
            fxAvailable ? t(payload.freshness || "delayed") : t("fxUnavailable"));
        $("fx-asof").textContent = t("fxAge", {
            date: payload.date,
            age: payload.source_age_days ?? "—",
            source: payload.source || "—"
        });
        const rates = $("fx-rates");
        rates.replaceChildren();
        for (const [currency, value] of Object.entries(payload.rates || {})) {
            const row = document.createElement("div"); row.className = "fx-row";
            const code = document.createElement("span"); code.textContent = t("reference", { currency });
            const rate = document.createElement("strong"); rate.textContent = numericFormat(value);
            row.append(code, rate); rates.append(row);
        }
        const chfToEur = payload.rates?.CHF;
        if (Number.isFinite(chfToEur) && chfToEur > 0) {
            const row = document.createElement("div"); row.className = "fx-row";
            const code = document.createElement("span"); code.textContent = t("inverseReference");
            const rate = document.createElement("strong"); rate.textContent = numericFormat(1 / chfToEur);
            row.append(code, rate); rates.append(row);
        }
        drawPriceChart();
    }

    async function refreshHistory() {
        const requestGeneration = window.pravdaAuthGeneration;
        const symbol = $("chart-symbol").value;
        const exchange = $("chart-exchange").value;
        const period = $("chart-period").value;
        const query = new URLSearchParams({ symbol, exchange, period });
        $("history-export").href = "#";
        try {
            const payload = await fetchJson(`/api/market/history?${query}`);
            historicalPoints = payload.observations || [];
            const ticks = historicalPoints.reduce((total, point) => total + (point.ticks || 1), 0);
            $("history-status").textContent = historicalPoints.length
                ? t("historyReady", { count: historicalPoints.length })
                : t("historyEmpty");
            drawPriceChart();
            return ticks;
        } catch (error) {
            if (requestGeneration !== window.pravdaAuthGeneration) return;
            $("history-status").textContent = t("unavailableFeed");
            historicalPoints = [];
            drawPriceChart();
            return 0;
        }
    }

    function renderAlerts(alerts) {
        const list = $("paper-alerts");
        list.replaceChildren();
        if (!alerts.length) {
            list.textContent = t("noAlerts");
            return;
        }
        for (const alert of alerts) {
            const row = document.createElement("div");
            row.className = "alert-row";
            const label = document.createElement("span");
            // The Rust API emits paper-simulation events (message/timestamp)
            // rather than editable price-threshold records. Never try to format
            // missing currency/target fields, or create a fake delete control.
            const isPriceAlert = typeof alert.id === "string"
                && Number.isFinite(alert.target)
                && ["EUR", "CHF"].includes(alert.currency)
                && ["above", "below"].includes(alert.direction);
            label.textContent = isPriceAlert
                ? `${safeText(alert.symbol)} · ${safeText(alert.exchange)} · ${t(alert.direction)} ${formatCurrency(alert.target, alert.currency)}`
                : `${safeText(alert.symbol)} · ${safeText(alert.message)} · ${formatDate(alert.timestamp)}`;
            row.append(label);
            if (isPriceAlert) {
                const remove = document.createElement("button");
                remove.className = "button";
                remove.type = "button";
                remove.textContent = t("deleteAlert");
                remove.addEventListener("click", () => {
                    $("alert-notice").textContent = t("unavailable");
                });
                row.append(remove);
            }
            list.append(row);
        }
    }

    async function refreshAlerts() {
        const requestGeneration = window.pravdaAuthGeneration;
        // Simulator events are read-only Rust snapshots. Editable thresholds
        // remain in a separate local-only list and never create orders.
        try {
            const payload = await fetchJson("/api/alerts");
            renderAlerts(Array.isArray(payload.alerts) ? payload.alerts : []);
        } catch (error) {
            if (requestGeneration !== window.pravdaAuthGeneration) return;
            $("paper-alerts").textContent = t("unavailableFeed");
        }
        showLocalAlerts();
    }

    async function checkPriceAlerts() {
        const requestGeneration = window.pravdaAuthGeneration;
        try {
            const payload = await fetchJson("/api/alerts/check");
            for (const alert of payload.triggered || []) {
                const message = t("alertTriggered", {
                    symbol: alert.symbol,
                    target: numericFormat(alert.target),
                    currency: alert.currency
                });
                $("alert-notice").textContent = message;
                if ("Notification" in window && Notification.permission === "granted") {
                    new Notification("Pravda-Bot", { body: message });
                }
            }
        } catch (error) {
            if (requestGeneration !== window.pravdaAuthGeneration) return;
            $("alert-notice").textContent = t("alertFailure");
        }
    }

    function renderSources(payload) {
        const list = $("source-status-list");
        list.replaceChildren();
        const sources = payload.sources || [];
        setStatus($("sources-status"), sources.some(source => source.status === "unavailable") ? "stale" : "live", `${sources.length}`);
        for (const source of sources) {
            const row = document.createElement("div");
            row.className = "source-row";
            const detail = document.createElement("div");
            const name = document.createElement("strong");
            name.textContent = source.name || t("sourceUnavailable");
            const time = document.createElement("small");
            time.textContent = source.last_event_at
                ? t("sourceRetrieved", { date: formatDate(source.last_event_at) })
                : source.detail || "—";
            detail.append(name, document.createElement("br"), time);
            const status = document.createElement("span");
            const state = ["live", "delayed", "stale"].includes(source.status) ? source.status : "error";
            setStatus(status, state, t(source.status === "unavailable" ? "sourceUnavailable" : state));
            row.append(detail, status);
            list.append(row);
        }
    }

    async function refreshSources() {
        const requestGeneration = window.pravdaAuthGeneration;
        try {
            renderSources(await fetchJson("/api/sources"));
        } catch (error) {
            if (requestGeneration !== window.pravdaAuthGeneration) return;
            setStatus($("sources-status"), "error", t("sourceUnavailable"));
            $("source-status-list").textContent = t("unavailableFeed");
        }
    }

    function drawMap(rows) {
        const svg = $("flow-map");
        svg.replaceChildren();
        const NS = "http://www.w3.org/2000/svg";
        const points = rows.filter(row => Number.isFinite(row.map_x) && Number.isFinite(row.map_y));
        if (!points.length) {
            const text = document.createElementNS(NS, "text");
            text.setAttribute("x", "400"); text.setAttribute("y", "210");
            text.setAttribute("text-anchor", "middle"); text.setAttribute("class", "map-axis");
            text.textContent = t("noMapPoints");
            svg.append(text);
            return;
        }
        const xs = points.map(row => row.map_x), ys = points.map(row => row.map_y);
        const minX = Math.min(...xs), maxX = Math.max(...xs), minY = Math.min(...ys), maxY = Math.max(...ys);
        for (let i = 0; i <= 4; i++) {
            const line = document.createElementNS(NS, "line");
            line.setAttribute("x1", String(35 + i * 182)); line.setAttribute("x2", String(35 + i * 182));
            line.setAttribute("y1", "26"); line.setAttribute("y2", "397");
            line.setAttribute("stroke", "rgba(73,115,143,.15)"); svg.append(line);
        }
        const unique = new Map();
        for (const row of points) unique.set(`${row.point_key}:${row.direction}`, row);
        for (const row of unique.values()) {
            const circle = document.createElementNS(NS, "circle");
            const x = 48 + ((row.map_x - minX) / (maxX - minX || 1)) * 704;
            const y = 35 + (1 - (row.map_y - minY) / (maxY - minY || 1)) * 350;
            circle.setAttribute("cx", String(x)); circle.setAttribute("cy", String(y));
            circle.setAttribute("r", String(Math.max(4, Math.min(11, 4 + Math.log10(Math.abs(row.value) + 1)))));
            circle.setAttribute("class", `map-point ${row.direction === "exit" ? "exit" : "entry"}`);
            const title = document.createElementNS(NS, "title");
            const directionLabel = row.direction === "exit" ? t("exit") : t("entry");
            title.textContent = `${row.point} · ${directionLabel} · ${numericFormat(row.value)} ${row.unit}`;
            circle.append(title); svg.append(circle);
        }
    }

    function renderFlows(payload) {
        const rows = payload.observations || [];
        const entries = rows.filter(row => row.direction === "entry");
        const exits = rows.filter(row => row.direction === "exit");
        $("entry-count").textContent = numericFormat(payload.entry_count ?? entries.length);
        $("exit-count").textContent = numericFormat(payload.exit_count ?? exits.length);
        $("entry-foot").textContent = t("inboundNote");
        $("exit-foot").textContent = t("outboundNote");
        $("source-update").textContent = formatDate(payload.latest_source_update);
        $("flow-status").textContent = payload.status === "live" ? t("recentPublication") : payload.status === "stale" ? t("oldPublication") : t("noUsableObservation");
        setStatus($("map-status"), payload.status === "live" ? "live" : "stale", payload.status === "live" ? t("recentSourceData") : t("delayedSourceData"));
        $("flow-notice").textContent = t("flowDisclaimer");
        drawMap(rows);
        const container = $("flow-rows");
        container.replaceChildren();
        if (!rows.length) {
            const empty = document.createElement("div");
            empty.className = "empty";
            empty.textContent = payload.status === "stale" ? t("staleFlowUnavailable") : t("noFlowObservation");
            container.append(empty);
            return;
        }
        for (const row of rows.slice(0, 14)) {
            const item = document.createElement("div");
            item.className = "flow-row";
            const name = document.createElement("span");
            name.className = "flow-name";
            name.textContent = row.point || row.point_key;
            name.title = `${row.operator || ""} ${row.country || ""}`.trim();
            const direction = document.createElement("span");
            direction.className = `direction ${row.direction === "exit" ? "exit" : ""}`;
            direction.textContent = row.direction === "exit" ? t("exitDirection") : t("entryDirection");
            const value = document.createElement("span");
            value.className = "flow-value";
            value.textContent = `${numericFormat(row.value)} ${row.unit}`;
            const time = document.createElement("span");
            time.className = "flow-sub flow-updated";
            time.textContent = formatDate(row.period_from);
            time.title = t("sourceUpdate", { date: formatDate(row.updated_at) });
            item.append(name, direction, value, time);
            container.append(item);
        }
    }

    // Map only implemented Rust IPC contracts; others remain unavailable.
    async function fetchJson(url) {
        const parsed = new URL(url, "https://pravda.invalid");
        const query = {
            symbol: parsed.searchParams.get("symbol") || "BTC/USDT",
            exchange: parsed.searchParams.get("exchange") || "Binance",
            period: parsed.searchParams.get("period") || "day"
        };
        const commands = {
            "/api/market": "ipc_market_snapshot",
            "/api/history": "ipc_history_recent",
            "/api/market/history": "ipc_history_query",
            "/api/alerts": "ipc_paper_alerts_snapshot",
            "/api/metrics": "ipc_runtime_metrics",
            "/api/fx": "ipc_fx_status",
            "/api/eu/flows": "ipc_public_feed",
            "/api/geopolitics": "ipc_public_feed",
            "/api/sources": "ipc_public_feed",
            "/api/revolut/public": "ipc_public_feed"
        };
        const command = commands[parsed.pathname];
        if (!command) throw new Error("UNAVAILABLE");
        // A response from session A must never relock a new authenticated
        // dashboard B or render stale data after a logout/login transition.
        const authGeneration = window.pravdaAuthGeneration;
        try {
            const publicSources = {"/api/eu/flows":"energy", "/api/geopolitics":"news",
                "/api/sources":"sources", "/api/revolut/public":"revolut"};
            const args = command === "ipc_public_feed" ? {source:publicSources[parsed.pathname]}
                : parsed.pathname === "/api/market/history" ? { query } : {};
            const result = await window.pravdaInvoke(command, args);
            if (authGeneration !== window.pravdaAuthGeneration) {
                throw new Error("STALE_SESSION");
            }
            return result;
        } catch (error) {
            // Reject a reply from a previous login before any caller can render it.
            if (authGeneration !== window.pravdaAuthGeneration) {
                throw new Error("STALE_SESSION");
            }
            if (String(error).includes("UNAUTHORIZED")) {
                window.pravdaRequireLogin();
            }
            throw error;
        }
    }
    let marketRequestNumber = 0;
    let lastSettledMarketRequest = 0;
    async function refreshMarket() {
        const requestGeneration = window.pravdaAuthGeneration;
        const requestNumber = ++marketRequestNumber;
        try {
            const payload = await fetchJson("/api/market");
            if (requestGeneration !== window.pravdaAuthGeneration) return;
            if (requestNumber < lastSettledMarketRequest) return;
            lastSettledMarketRequest = requestNumber;
            renderMarket(payload);
            // History has a separate, slower polling cadence to avoid repeated SQLite reads.
        }
        catch (error) {
            if (requestGeneration !== window.pravdaAuthGeneration) return;
            if (requestNumber < lastSettledMarketRequest) return;
            lastSettledMarketRequest = requestNumber;
            clearMarketSnapshot();
        }
    }
    // Auth boundaries invalidate pending IPC results and any cached consent UI.
    let revolutOptIn = false;
    let revolutConsentVersion = 0;
    const revolutConsentCheckbox = $("revolut-consent");
    const revolutConsentStatus = $("revolut-consent-status");
    const revolutReadButton = $("revolut-refresh");

    function lockRevolutConsentUi() {
        revolutConsentVersion++;
        revolutOptIn = false;
        revolutConsentCheckbox.checked = false;
        revolutConsentCheckbox.disabled = true;
        revolutReadButton.disabled = true;
        revolutConsentStatus.textContent = "Session verrouillée : consentement non consultable.";
        $("revolut-orderbook").textContent = "Lecture facultative verrouillée.";
        setStatus($("revolut-status"), "pending", "Verrouillé");
    }
    window.pravdaLockRevolutConsent = lockRevolutConsentUi;

    async function readRevolutConsent() {
        const version = ++revolutConsentVersion;
        revolutOptIn = false;
        revolutConsentCheckbox.checked = false;
        revolutConsentCheckbox.disabled = true;
        revolutReadButton.disabled = true;
        try {
            const enabled = await window.pravdaInvoke("ipc_revolut_consent_get");
            if (version !== revolutConsentVersion || shell.hidden) return;
            if (typeof enabled !== "boolean") throw new Error("INVALID_CONSENT");
            revolutOptIn = enabled;
            revolutConsentCheckbox.checked = enabled;
            revolutConsentCheckbox.disabled = false;
            revolutReadButton.disabled = !enabled;
            revolutConsentStatus.textContent = enabled
                ? "Lecture publique autorisée. Aucun compte ou ordre réel connecté."
                : "Désactivé : aucune lecture du carnet public Revolut X.";
        } catch {
            if (version !== revolutConsentVersion || shell.hidden) return;
            revolutOptIn = false;
            revolutConsentCheckbox.checked = false;
            revolutReadButton.disabled = true;
            revolutConsentStatus.textContent = "Consentement indisponible : carnet désactivé.";
        }
    }
    window.refreshRevolutConsent = readRevolutConsent;

    revolutConsentCheckbox.addEventListener("change", async () => {
        if (shell.hidden) {
            lockRevolutConsentUi();
            return;
        }
        const version = ++revolutConsentVersion;
        const requested = revolutConsentCheckbox.checked;
        revolutConsentCheckbox.disabled = true;
        revolutReadButton.disabled = true;
        try {
            const stored = await window.pravdaInvoke("ipc_revolut_consent_set", {enabled: requested});
            if (version !== revolutConsentVersion || shell.hidden) return;
            if (stored !== requested) throw new Error("CONSENT_NOT_PERSISTED");
            revolutOptIn = stored;
            revolutConsentCheckbox.checked = stored;
            revolutConsentStatus.textContent = stored
                ? "Lecture publique autorisée. Aucun compte ou ordre réel connecté."
                : "Désactivé : aucune lecture du carnet public Revolut X.";
            if (!stored) {
                $("revolut-orderbook").textContent = "Module facultatif désactivé.";
                setStatus($("revolut-status"), "pending", "Désactivé");
            }
        } catch {
            if (version !== revolutConsentVersion || shell.hidden) return;
            revolutOptIn = false;
            revolutConsentCheckbox.checked = false;
            revolutConsentStatus.textContent = "Préférence non enregistrée : aucune lecture autorisée.";
        } finally {
            if (version === revolutConsentVersion && !shell.hidden) {
                revolutConsentCheckbox.disabled = false;
                revolutReadButton.disabled = !revolutOptIn;
            }
        }
    });
    readRevolutConsent();

    async function refreshRevolut() {
        const requestGeneration = window.pravdaAuthGeneration;
        const status = $("revolut-status");
        const container = $("revolut-orderbook");
        if (!revolutOptIn) {
            container.textContent = "Module facultatif désactivé.";
            setStatus(status, "pending", "Désactivé");
            return;
        }
        setStatus(status,"pending","Lecture du carnet public…");
        try {
            const data = await fetchJson("/api/revolut/public");
            if (data.status === "unavailable" || !data.book?.data) throw new Error("PUBLIC_ORDER_BOOK_UNAVAILABLE");
            const book = data.book.data;
            const bestBid = book.bids?.[0]?.price;
            const bestAsk = book.asks?.[0]?.price;
            if (!bestBid || !bestAsk) throw new Error("EMPTY_BOOK");
            container.replaceChildren();
            for(const [label,price] of [["Meilleure offre d'achat",bestBid],["Meilleure offre de vente",bestAsk]]) {
                const panel = document.createElement("div"); panel.className="connector-metric";
                const text = document.createElement("span"); text.textContent=label;
                const value=document.createElement("strong");value.textContent=String(price)+" USD";
                panel.append(text,value);container.append(panel);
            }
            setStatus(status,"live","Carnet public reçu");
        } catch (error) {
            if (requestGeneration !== window.pravdaAuthGeneration) return;
            container.textContent="Le carnet public Revolut X est indisponible. Aucun prix fictif n’est affiché.";
            setStatus(status,"error","Source indisponible");
        }
    }
    async function refreshSystem() {
        const requestGeneration = window.pravdaAuthGeneration;
        try {
            const d = await fetchJson("/api/metrics");
            const fmt=(v,unit="")=>Number.isFinite(v)?numericFormat(v)+unit:"—";
            $("runtime-cpu").textContent=fmt(d.cpu_percent,"%");
            $("runtime-ram").textContent=fmt(d.memory_usage_mb," MiB");
            $("runtime-uptime").textContent=fmt(d.uptime_seconds," s");
            $("runtime-pid").textContent=d.pid??"—";
            setStatus($("runtime-status"),d.status==="available"?"live":"pending",
                d.status==="available"?"Mesures locales":"Mesures partielles");
        } catch (error) {
            if (requestGeneration !== window.pravdaAuthGeneration) return;
            setStatus($("runtime-status"),"error","Diagnostic indisponible");
        }
    }
    // Never request protected diagnostics in the background after logout.
    // A SQLite quick_check is user-initiated or runs when Security is opened.
    const securityShell = document.querySelector("main.shell");
    // Only passive diagnostics require a foreground document. Explicit UI
    // clicks must still run on WebView2 if visibilityState is stale or hidden.
    // The Rust session/window check is the real authorization boundary.
    const securitySessionReady = () => !!securityShell && !securityShell.hidden;
    const securityVisible = () => securitySessionReady() && !document.hidden;
    let securityCheckInFlight = false;
    async function refreshSecurityEvents() {
        const requestGeneration = window.pravdaAuthGeneration;
        const panel = $("security-events");
        const labels = {
            setup_accepted: "Premier mot de passe configuré",
            setup_rejected: "Configuration du mot de passe refusée",
            login_accepted: "Connexion locale réussie",
            login_rejected: "Tentative de connexion refusée",
            login_locked: "Authentification temporairement verrouillée",
            session_ended: "Session fermée",
            storage_failure: "Erreur d'authentification ou de stockage"
        };
        try {
            const events = await window.pravdaInvoke("ipc_security_events");
            if (requestGeneration !== window.pravdaAuthGeneration) return;
            if (!Array.isArray(events) || events.length > 64) throw new Error("INVALID_EVENTS");
            const nodes = [];
            for (const line of events) {
                if (typeof line !== "string" || line.length > 70) throw new Error("INVALID_EVENT");
                const parts = line.split("|");
                if (parts.length !== 2 || !/^\d{1,11}$/.test(parts[0]) ||
                    !Object.hasOwn(labels, parts[1])) throw new Error("INVALID_EVENT");
                const unix = Number(parts[0]);
                if (!Number.isSafeInteger(unix)) throw new Error("INVALID_TIME");
                const item = document.createElement("div");
                item.className = "security-check";
                item.setAttribute("role", "listitem");
                const heading = document.createElement("strong");
                heading.textContent = labels[parts[1]];
                const time = document.createElement("p");
                time.textContent = new Date(unix * 1000).toLocaleString("fr-FR");
                item.append(heading, time);
                nodes.push(item);
            }
            if (nodes.length) panel.replaceChildren(...nodes);
            else panel.textContent = "Aucun événement enregistré depuis le lancement.";
        } catch (error) {
            if (requestGeneration !== window.pravdaAuthGeneration) return;
            panel.textContent = "Historique de sécurité non disponible. Réauthentification nécessaire.";
        }
    }
    async function refreshSecurity(explicit = false) {
        const requestGeneration = window.pravdaAuthGeneration;
        if (!securitySessionReady()) {
            if (explicit) setStatus($("security-status"), "error", "Connexion locale requise");
            return;
        }
        if (!explicit && !securityVisible()) return;
        if (securityCheckInFlight) {
            if (explicit) setStatus($("security-status"), "pending", "Vérification déjà en cours…");
            return;
        }
        securityCheckInFlight = true;
        const panel = $("security-checks");
        if (explicit) setStatus($("security-status"), "pending", "Vérification locale en cours…");
        try {
            const rows = await window.pravdaInvoke("ipc_security_report");
            if (requestGeneration !== window.pravdaAuthGeneration) return;
            if (!Array.isArray(rows) || rows.length !== 8) throw new Error("INVALID_REPORT");
            const translated = {
                verified: "Vérifié",
                unknown: "Non vérifié",
                warning: "Attention",
                blocked: "Bloqué"
            };
            const checkNames = {
                session: "Authentification Rust",
                sqlite: "Intégrité SQLite",
                authenticity: "Authenticité du binaire",
                capabilities: "Permissions IPC et CSP",
                market: "Données de marché",
                orders: "Ordres et sécurité financière",
                proton: "Proton VPN",
                kill_switch: "Protection anti-fuite VPN"
            };
            const nodes = [];
            let verified = 0;
            let blocked = 0;
            for (const row of rows) {
                if (typeof row !== "string" || row.length > 300) throw new Error("INVALID_ROW");
                const parts = row.split("|");
                if (parts.length !== 3 || !translated[parts[1]] ||
                    !Object.hasOwn(checkNames, parts[0])) throw new Error("INVALID_ROW");
                if (parts[1] === "verified") verified++;
                if (parts[1] === "blocked") blocked++;
                const item = document.createElement("div");
                item.className = "security-check";
                item.setAttribute("role", "listitem");
                const header = document.createElement("strong");
                header.textContent = checkNames[parts[0]] + " · " + translated[parts[1]];
                const detail = document.createElement("p");
                detail.textContent = parts[2];
                item.append(header, detail);
                nodes.push(item);
            }
            panel.replaceChildren(...nodes);
            setStatus($("security-status"), blocked ? "error" : "pending",
                blocked ? blocked + " blocage(s) de sécurité" :
                    verified + " / " + rows.length + " contrôles vérifiés ; audit incomplet" +
                    " · " + new Date().toLocaleTimeString("fr-FR"));
            $("security-status").title = "Dernière vérification locale : " + new Date().toLocaleString("fr-FR");
        } catch (error) {
            if (requestGeneration !== window.pravdaAuthGeneration) return;
            panel.textContent = "État de sécurité non disponible. Vérifiez votre session locale.";
            setStatus($("security-status"), "error", "Non vérifié");
        } finally {
            securityCheckInFlight = false;
        }
    }
    // Native, authenticated pre/postproduction tests belong to the installed
    // desktop. No test center executable, shell invocation or HTTP endpoint.
    let acceptanceBusy = false;
    let acceptanceRuns = 0;
    const expectedAcceptanceChecks = [
        "session", "ipc", "paper", "sqlite", "webview",
        "installer", "persistence", "authenticode", "live"
    ];
    const acceptanceLabels = {
        session: "Authentification Rust", ipc: "IPC de la fenêtre",
        paper: "Simulation fictive", sqlite: "Intégrité SQLite",
        webview: "Tauri / WebView2", installer: "Installation hors réseau",
        persistence: "Redémarrage et restauration", authenticode: "Signature",
        live: "Trading réel"
    };
    function clearAcceptanceUi() {
        acceptanceRuns = 0;
        $("acceptance-results").textContent = "Aucun résultat disponible.";
        $("acceptance-status").textContent = "Aucune campagne lancée.";
        $("acceptance-json").value = "Aucun rapport généré.";
    }
    window.pravdaClearAcceptance = clearAcceptanceUi;
    async function runAcceptance(phase) {
        const requestGeneration = window.pravdaAuthGeneration;
        if (!["preproduction", "postproduction"].includes(phase)) return;
        if (!securitySessionReady()) {
            $("acceptance-status").textContent = "Connectez-vous pour exécuter ces contrôles.";
            return;
        }
        if (acceptanceBusy) {
            $("acceptance-status").textContent = "Campagne précédente encore en cours.";
            return;
        }
        acceptanceBusy = true;
        const runNumber = ++acceptanceRuns;
        $("acceptance-pre").disabled = true;
        $("acceptance-post").disabled = true;
        $("acceptance-status").textContent =
            "Campagne n°" + runNumber + " : contrôles Rust locaux en cours…";
        try {
            const rows = await window.pravdaInvoke("ipc_acceptance_report", {phase});
            if (requestGeneration !== window.pravdaAuthGeneration) return;
            if (!Array.isArray(rows) || rows.length !== expectedAcceptanceChecks.length)
                throw new Error("INVALID_ACCEPTANCE_REPORT");
            let verified = 0, blocked = 0, unknown = 0;
            const nodes = [];
            for (const [index, line] of rows.entries()) {
                if (typeof line !== "string" || line.length > 250) throw new Error("INVALID_ACCEPTANCE_ROW");
                const parts = line.split("|");
                if (parts.length !== 3 || parts[0] !== expectedAcceptanceChecks[index] ||
                    !Object.hasOwn(acceptanceLabels, parts[0]) ||
                    !["verified", "blocked", "unknown"].includes(parts[1])) {
                    throw new Error("INVALID_ACCEPTANCE_ROW");
                }
                if (parts[1] === "verified") verified++;
                if (parts[1] === "blocked") blocked++;
                if (parts[1] === "unknown") unknown++;
                const item = document.createElement("div");
                item.className = "security-check";
                item.setAttribute("role", "listitem");
                const header = document.createElement("strong");
                header.textContent = acceptanceLabels[parts[0]] + " · " +
                    {verified: "Vérifié", blocked: "Échec", unknown: "Non vérifié"}[parts[1]];
                const detail = document.createElement("p");
                detail.textContent = parts[2];
                item.append(header, detail);
                nodes.push(item);
            }
            $("acceptance-results").replaceChildren(...nodes);
            const finishedAt = new Date();
            $("acceptance-status").textContent =
                (phase === "preproduction" ? "Préproduction" : "Postproduction") +
                " · exécution n°" + runNumber + " à " +
                finishedAt.toLocaleTimeString("fr-FR") + " : " +
                verified + " contrôles locaux vérifiés sur " + rows.length +
                ", " + blocked + " échec(s), " + unknown +
                " non vérifiés. Ne constitue pas une homologation.";
            $("acceptance-json").value = JSON.stringify({
                schema_version: 1, phase, run_number: runNumber,
                checked_at_utc: finishedAt.toISOString(),
                verified, blocked, unknown, findings: rows
            }, null, 2);
        } catch (_error) {
            if (requestGeneration !== window.pravdaAuthGeneration) return;
            $("acceptance-results").textContent = "Rapport non disponible.";
            $("acceptance-status").textContent = "Échec de contrôle. Aucun PASS déduit.";
            $("acceptance-json").value = "Aucun rapport valide généré.";
        } finally {
            acceptanceBusy = false;
            $("acceptance-pre").disabled = false;
            $("acceptance-post").disabled = false;
        }
    }
    $("acceptance-pre").addEventListener("click", () => runAcceptance("preproduction"));
    $("acceptance-post").addEventListener("click", () => runAcceptance("postproduction"));
    $("security-refresh").addEventListener("click", () => {
        refreshSecurity(true);
        refreshSecurityEvents();
    });
    window.refreshSecurity = () => refreshSecurity(true);
    window.refreshSecurityEvents = refreshSecurityEvents;
    refreshSecurity();
    refreshSecurityEvents();

    // Paper Trading is a deterministic, Rust-owned sandbox and never touches
    // authenticated exchanges, local browser storage, or actual account funds.
    const paperFormat = cents => (cents / 100).toLocaleString("fr-FR", {
        minimumFractionDigits: 2, maximumFractionDigits: 2
    }) + " USDT fictifs";
    const paperNumber = value => {
        if (!/^\d{1,15}$/.test(value)) throw new Error("INVALID_PAPER_RESULT");
        const number = Number(value);
        if (!Number.isSafeInteger(number)) throw new Error("INVALID_PAPER_RESULT");
        return number;
    };
    function renderPaper(rows) {
        if (!Array.isArray(rows) || rows.length > 40) throw new Error("INVALID_PAPER_RESULT");
        const state = {mode:"", cash:null, equity:null, fee:null, priceType:"",
            prices:{}, positions:{}, events:[]};
        for (const raw of rows) {
            if (typeof raw !== "string" || raw.length > 120) throw new Error("INVALID_PAPER_RESULT");
            const parts = raw.split("|");
            switch (parts[0]) {
                case "mode":
                    if (parts.length !== 2) throw new Error("INVALID_PAPER_RESULT");
                    state.mode = parts[1]; break;
                case "cash_cents":
                    if (parts.length !== 2) throw new Error("INVALID_PAPER_RESULT");
                    state.cash = paperNumber(parts[1]); break;
                case "equity_cents":
                    if (parts.length !== 2) throw new Error("INVALID_PAPER_RESULT");
                    state.equity = paperNumber(parts[1]); break;
                case "fee_bps":
                    if (parts.length !== 2) throw new Error("INVALID_PAPER_RESULT");
                    state.fee = paperNumber(parts[1]); break;
                case "price_type":
                    if (parts.length !== 2) throw new Error("INVALID_PAPER_RESULT");
                    state.priceType = parts[1]; break;
                case "price_cents":
                    if (parts.length !== 3 || !["BTC","ETH","SOL"].includes(parts[1]))
                        throw new Error("INVALID_PAPER_RESULT");
                    state.prices[parts[1]] = paperNumber(parts[2]); break;
                case "position_milli":
                    if (parts.length !== 3 || !["BTC","ETH","SOL"].includes(parts[1]))
                        throw new Error("INVALID_PAPER_RESULT");
                    state.positions[parts[1]] = paperNumber(parts[2]); break;
                case "event":
                    if (parts.length !== 7 || !["BUY","SELL"].includes(parts[2]) ||
                        !["BTC","ETH","SOL"].includes(parts[3]))
                        throw new Error("INVALID_PAPER_RESULT");
                    state.events.push({seq:paperNumber(parts[1]),side:parts[2],symbol:parts[3],
                        quantity:paperNumber(parts[4]),gross:paperNumber(parts[5]),
                        fee:paperNumber(parts[6])});
                    break;
                default: throw new Error("INVALID_PAPER_RESULT");
            }
        }
        if (state.mode !== "PAPER_ONLY" || state.priceType !== "SYNTHETIC_FIXED" ||
            state.cash === null || state.equity === null || state.fee !== 36 ||
            ["BTC","ETH","SOL"].some(symbol =>
                !Number.isSafeInteger(state.prices[symbol]) ||
                !Number.isSafeInteger(state.positions[symbol])) ||
            state.events.length > 20) throw new Error("INVALID_PAPER_RESULT");
        $("paper-summary").textContent =
            "Solde : " + paperFormat(state.cash) + " · Équité : " + paperFormat(state.equity);
        const positions = ["BTC","ETH","SOL"].map(symbol => {
            const node = document.createElement("div");
            node.textContent = symbol + " · " + (state.positions[symbol] / 1000).toFixed(3)
                + " unité fictive · cotation fixe " + paperFormat(state.prices[symbol]);
            return node;
        });
        $("paper-positions").replaceChildren(...positions);
        const events = state.events.map(item => {
            const node = document.createElement("div");
            node.setAttribute("role","listitem");
            node.textContent = "#" + item.seq + " " +
                (item.side === "BUY" ? "Achat" : "Vente") + " simulé · "
                + (item.quantity / 1000).toFixed(3) + " " + item.symbol
                + " · montant " + paperFormat(item.gross)
                + " · frais " + paperFormat(item.fee);
            return node;
        });
        if (events.length) $("paper-events").replaceChildren(...events);
        else $("paper-events").textContent = "Aucune transaction simulée.";
    }
    function clearPaperUi() {
        $("paper-summary").textContent = "Portefeuille fictif non chargé.";
        $("paper-positions").textContent = "Positions fictives non chargées.";
        $("paper-events").textContent = "Aucune transaction simulée chargée.";
        $("paper-notice").textContent = "Simulation disponible après authentification.";
    }
    window.pravdaClearPaper = clearPaperUi;
    let paperBusy = false;
    function paperMessage(error) {
        const messages = {
            INVALID_PAPER_INSTRUMENT:"Instrument fictif non autorisé.",
            INVALID_PAPER_SIDE:"Action non autorisée.",
            INVALID_PAPER_QUANTITY:"Quantité simulée incorrecte.",
            PAPER_RISK_LIMIT:"Limite de risque : 2 500 USDT fictifs par opération.",
            PAPER_INSUFFICIENT_FUNDS:"Solde fictif insuffisant.",
            PAPER_INSUFFICIENT_POSITION:"Position fictive insuffisante.",
            PAPER_ARITHMETIC_ERROR:"Opération bloquée : contrôle arithmétique.",
            UNAUTHORIZED:"Session expirée : reconnectez-vous."
        };
        return messages[String(error)] || "Simulation locale indisponible.";
    }
    async function refreshPaper(explicit = false) {
        const generation = window.pravdaAuthGeneration;
        if (!securitySessionReady() || (!explicit && !securityVisible()) || paperBusy) return;
        try {
            const rows = await window.pravdaInvoke("ipc_paper_snapshot");
            if (generation !== window.pravdaAuthGeneration) return;
            renderPaper(rows);
            $("paper-notice").textContent =
                "Mode PAPER ONLY · prix fixes synthétiques · aucune connexion au marché.";
        } catch (error) {
            if (generation !== window.pravdaAuthGeneration) return;
            $("paper-notice").textContent = paperMessage(error);
        }
    }
    async function paperAction(command, args = {}) {
        if (paperBusy) return;
        const generation = window.pravdaAuthGeneration;
        paperBusy = true;
        $("paper-trade").disabled = true;
        $("paper-reset").disabled = true;
        try {
            const rows = await window.pravdaInvoke(command, args);
            if (generation !== window.pravdaAuthGeneration) return;
            renderPaper(rows);
            $("paper-notice").textContent = command === "ipc_paper_reset"
                ? "Portefeuille fictif remis à zéro. Aucun compte réel affecté."
                : "Opération PAPER simulée en mémoire. Aucun ordre réel envoyé.";
        } catch (error) {
            if (generation !== window.pravdaAuthGeneration) return;
            $("paper-notice").textContent = paperMessage(error);
        } finally {
            paperBusy = false;
            $("paper-trade").disabled = false;
            $("paper-reset").disabled = false;
            // An old response may finish after a new Rust login reset the portfolio.
            // Fetch the current session snapshot instead of leaving a blank/stale view.
            if (generation !== window.pravdaAuthGeneration) refreshPaper(true);
        }
    }
    $("paper-order-form").addEventListener("submit", event => {
        event.preventDefault();
        paperAction("ipc_paper_trade", {
            symbol:$("paper-symbol").value,
            side:$("paper-side").value,
            quantityMilli:Number($("paper-quantity").value)
        });
    });
    $("paper-reset").addEventListener("click", () => paperAction("ipc_paper_reset"));
    window.refreshPaper = () => refreshPaper(true);
    refreshPaper(true);

    // No unconditional security polling: SQLite diagnostics are only requested
    // on login, an explicit refresh or when the Security tab is opened.
    document.addEventListener("click",async event => {
        const tab=event.target.closest("[data-go-tab]");
        if(tab){document.querySelector('.tab-btn[data-tab="'+tab.dataset.goTab+'"]')?.click();return;}
        const anchor=event.target.closest("a[data-source-link], a[data-article-url]");
        if(!anchor)return;
        event.preventDefault();
        const msg=$("source-link-notice");
        try{
            const args=anchor.dataset.articleUrl ? {source:"article",link:anchor.href}
                : {source:anchor.dataset.sourceLink};
            await window.pravdaInvoke("ipc_open_source",args);
            msg.textContent="Source ouverte dans le navigateur Windows";
        }catch{msg.textContent="Impossible d’ouvrir la source officielle. Vérifiez votre navigateur par défaut.";}
    });
    $("revolut-refresh").addEventListener("click",refreshRevolut);
    async function refreshFx() {
        const requestGeneration = window.pravdaAuthGeneration;
        try { renderFx(await fetchJson("/api/fx")); }
        catch (error) {
            if (requestGeneration !== window.pravdaAuthGeneration) return;
            // Old FX rates must not be reused for conversions or replayed by
            // a later language change when the current request has failed.
            latestFx = null;
            setStatus($("fx-status"), "error", t("fxUnavailable"));
            $("fx-rates").textContent = t("fxFailure");
            $("fx-asof").textContent = "—";
            // USDT observations do not depend on FX. Keep the latest market
            // snapshot, but redraw it without any EUR/CHF conversion.
            if (latestMarket) renderMarket(latestMarket);
            else drawPriceChart();
        }
    }
    async function refreshFlows() {
        const requestGeneration = window.pravdaAuthGeneration;
        try {
            const data=await fetchJson("/api/eu/flows");
            if(data.status==="unavailable")throw new Error(data.notice || "SOURCE_UNAVAILABLE");
            renderFlows(data);
        }
        catch (error) {
            if (requestGeneration !== window.pravdaAuthGeneration) return;
            setStatus($("map-status"), "error", t("unavailable"));
            $("flow-status").textContent = t("unavailable");
            $("flow-rows").textContent = t("flowFailure");
        }
    }
    async function refreshNews() {
        const requestGeneration = window.pravdaAuthGeneration;
        const list = $("news-list");
        try {
            const payload = await fetchJson("/api/geopolitics");
            if(payload.status==="unavailable" || !Array.isArray(payload.articles))
                throw new Error(payload.notice || "NEWS_UNAVAILABLE");
            const sourceName = payload.source?.startsWith("BBC") ? "BBC World" : "GDELT";
            setStatus($("news-status"), "live", `${sourceName} · ${payload.articles.length}`);
            $("news-source").textContent = `${t("newsWindow")} · ${t("publishedOn", { date: formatDate(payload.retrieved_at) })}`;
            list.replaceChildren();
            if (!payload.articles.length) {
                list.textContent = t("noArticles");
                return;
            }
            for (const article of payload.articles) {
                let articleUrl;
                try { articleUrl = new URL(article.url); }
                catch { continue; }
                if (!["http:", "https:"].includes(articleUrl.protocol)) continue;
                const item = document.createElement("article");
                item.className = "news-item";
                const link = document.createElement("a");
                link.href = articleUrl.href;
                link.dataset.articleUrl = "true";
                link.target = "_blank"; link.rel = "noopener noreferrer";
                link.textContent = article.title;
                const meta = document.createElement("div");
                meta.className = "news-meta";
                const source = document.createElement("span");
                source.textContent = article.source || t("mediaSource");
                const date = document.createElement("time");
                date.textContent = formatDate(article.published_at);
                meta.append(source, date); item.append(link, meta); list.append(item);
            }
        } catch (error) {
            if (requestGeneration !== window.pravdaAuthGeneration) return;
            setStatus($("news-status"), "error", t("unavailableFeed"));
            list.textContent = t("newsFailure");
        }
    }
    async function refreshCrypto() {
        const requestGeneration = window.pravdaAuthGeneration;
        try {
            const payload=await fetchJson("/api/market");
            const symbols=payload.symbols || {};
            const rows=Object.entries(symbols).flatMap(([symbol,exchanges])=>
                Object.entries(exchanges).map(([exchange,quote])=>({symbol,exchange,quote})));
            const live=rows.filter(row=>row.quote?.status==="live");
            setStatus($("bot-status"),live.length?"live":"stale",
                live.length?`${live.length} cotation(s) fraîche(s)`:"Aucune cotation récente");
            const container=$("crypto-data");container.replaceChildren();
            if(!rows.length){container.textContent="Aucune cotation observée. Le mode hors ligne reste disponible.";return;}
            const table=document.createElement("table");table.className="quote-table";
            const head=table.createTHead().insertRow();
            for(const label of ["Actif","Plateforme","Prix USDT","Fraîcheur"]){
                const th=document.createElement("th");th.textContent=label;head.append(th);
            }
            for(const row of rows.slice(0,12)){
                const tr=table.insertRow();
                for(const value of [row.symbol,row.exchange,
                    Number.isFinite(row.quote?.price)?numericFormat(row.quote.price)+" USDT":"—",
                    row.quote?.status==="live"?"Récent":"Périmé"]){
                    const td=tr.insertCell();td.textContent=value;
                }
            }
            container.append(table);
        } catch (error) {
            if (requestGeneration !== window.pravdaAuthGeneration) return;
            setStatus($("bot-status"),"error","Flux local indisponible");
            $("crypto-data").textContent="Lecture des cours indisponible ; aucun prix n'est inventé.";
        }
    }
    async function refreshAIS() {
        const requestGeneration = window.pravdaAuthGeneration;
        try {
            const payload = await fetchJson("/api/ais/recent?limit=50");
            setStatus($("ais-status"), "live", `${payload.count} ${t("aisTitle")}`);
            const container = $("ais-data");
            const mapSvg = $("ais-map");
            if (!payload.messages || !payload.messages.length) {
                container.textContent = t("aisLoading");
                if (mapSvg) { mapSvg.replaceChildren(); }
                return;
            }
            if (mapSvg) {
                const NS = "http://www.w3.org/2000/svg";
                mapSvg.replaceChildren();
                const msgs = payload.messages.filter(m => Number.isFinite(parseFloat(m.lat)) && Number.isFinite(parseFloat(m.lon)));
                if (msgs.length) {
                    const lats = msgs.map(m=>parseFloat(m.lat));
                    const lons = msgs.map(m=>parseFloat(m.lon));
                    const minLat = Math.min(...lats), maxLat = Math.max(...lats);
                    const minLon = Math.min(...lons), maxLon = Math.max(...lons);
                    const pad = 40;
                    msgs.forEach(m=>{
                        const lat = parseFloat(m.lat), lon = parseFloat(m.lon);
                        const x = pad + ((lon - minLon)/(maxLon - minLon || 1)) * (800 - pad*2);
                        const y = pad + (1 - (lat - minLat)/(maxLat - minLat || 1)) * (430 - pad*2);
                        const circle = document.createElementNS(NS,"circle");
                        circle.setAttribute("cx", String(x));
                        circle.setAttribute("cy", String(y));
                        circle.setAttribute("r", "4");
                        circle.setAttribute("fill", "#4d8edc");
                        circle.setAttribute("stroke", "#07111f");
                        circle.setAttribute("stroke-width","2");
                        const title = document.createElementNS(NS,"title");
                        title.textContent = `${m.mmsi ?? "?"} ${m.lat},${m.lon} ${m.sog ?? ""}kn`;
                        circle.append(title);
                        mapSvg.append(circle);
                    });
                } else {
                    const text = document.createElementNS(NS,"text");
                    text.setAttribute("x","400"); text.setAttribute("y","215"); text.setAttribute("text-anchor","middle"); text.setAttribute("fill","#9eacc2"); text.textContent = t("aisLoading");
                    mapSvg.append(text);
                }
            }
            container.replaceChildren();
            const table = document.createElement("table");
            const head = table.createTHead().insertRow();
            for (const label of ["MMSI", "Lat", "Lon", "SOG kn", "COG", "Timestamp", "Source"]) {
                const th = document.createElement("th"); th.textContent = label; head.append(th);
            }
            const body = table.createTBody();
            for (const m of payload.messages.slice(0,20)) {
                const tr = body.insertRow();
                tr.insertCell().textContent = m.mmsi ?? "—";
                tr.insertCell().textContent = m.lat ?? "—";
                tr.insertCell().textContent = m.lon ?? "—";
                tr.insertCell().textContent = m.sog ?? "—";
                tr.insertCell().textContent = m.cog ?? "—";
                tr.insertCell().textContent = m.timestamp ?? "—";
                tr.insertCell().textContent = m.source ?? "—";
            }
            container.replaceChildren(table);
        } catch (error) {
            if (requestGeneration !== window.pravdaAuthGeneration) return;
            setStatus($("ais-status"), "error", t("aisUnavailable"));
            $("ais-data").textContent = t("aisUnavailable");
        }
    }
    function updateClock() { $("clock").textContent = new Date().toLocaleTimeString(languageLocales[currentLanguage]); }
    let refreshInFlight = false;
    let refreshGenerationInFlight = null;
    let queuedAuthRefreshGeneration = null;
    async function refreshAll() {
        const button = $("refresh-button");
        const requestedGeneration = window.pravdaAuthGeneration;
        if (refreshInFlight) {
            // A login changes the Rust session generation. Do not discard its
            // refresh just because a previous session still has IPC in flight.
            if (requestedGeneration !== refreshGenerationInFlight) {
                queuedAuthRefreshGeneration = requestedGeneration;
                button.title = "Nouvelle session : actualisation en attente";
            } else {
                button.title = "Actualisation déjà en cours";
            }
            return;
        }
        refreshInFlight = true;
        refreshGenerationInFlight = requestedGeneration;
        button.disabled = true;
        button.textContent = "↻ Actualisation en cours…";
        $("daily-report").removeAttribute("href");
        try {
            const results = await Promise.allSettled([
                refreshMarket(), refreshFx(), refreshFlows(), refreshNews(), refreshCrypto(),
                refreshHistory(), refreshAlerts(), refreshSources(), refreshAIS()
            ]);
            const rejected = results.filter(result => result.status === "rejected").length;
            button.textContent = "↻ Tentative à " + new Date().toLocaleTimeString("fr-FR");
            button.title = rejected ? "Certaines sources sont indisponibles" :
                "Dernière tentative d'actualisation locale (les sources peuvent rester périmées)";
        } finally {
            refreshInFlight = false;
            refreshGenerationInFlight = null;
            button.disabled = false;
            const nextGeneration = queuedAuthRefreshGeneration;
            queuedAuthRefreshGeneration = null;
            if (nextGeneration !== null &&
                nextGeneration === window.pravdaAuthGeneration &&
                !$("shell").hidden) {
                // Re-run after releasing the guard. Old generation results are
                // already ignored individually by their IPC response guards.
                void refreshAll();
            }
        }
    }
    $("refresh-button").addEventListener("click", refreshAll);
    $("language-select").addEventListener("change", () => {
        currentLanguage = supportedLanguages.includes($("language-select").value) ? $("language-select").value : "fr";
        try {
            localStorage.setItem("pravda-dashboard-language", currentLanguage);
        } catch {
            $("language-select").value = currentLanguage;
        }
        applyLanguage();
        refreshAll();
    });
    $("chart-symbol").addEventListener("change", drawPriceChart);
    $("chart-symbol").addEventListener("change", refreshHistory);
    $("chart-exchange").addEventListener("change", refreshHistory);
    $("chart-period").addEventListener("change", refreshHistory);
    $("chart-currency").addEventListener("change", drawPriceChart);
    const alertStorageKey = "pravda-local-price-alerts-v1";
    let localAlerts = [];
    try {
        const parsed = JSON.parse(localStorage.getItem(alertStorageKey) || "[]");
        if (Array.isArray(parsed)) localAlerts = parsed.filter(a =>
            typeof a.id==="string" && ["EUR","CHF"].includes(a.currency) &&
            ["above","below"].includes(a.direction) && Number.isFinite(a.target) &&
            a.target>0 && ["BTC/USDT","ETH/USDT","SOL/USDT"].includes(a.symbol) &&
            ["Kraken","Binance"].includes(a.exchange)).slice(0,30);
    } catch { localAlerts=[]; }
    const storeLocalAlerts = () => {
        try { localStorage.setItem(alertStorageKey, JSON.stringify(localAlerts)); }
        catch { $("alert-notice").textContent="Stockage local inaccessible : alertes conservées en mémoire uniquement."; }
    };
    const showLocalAlerts = () => {
        const container = $("price-alerts");
        container.replaceChildren();
        if(!localAlerts.length){container.textContent="Aucune alerte locale configurée.";return;}
        for(const alert of localAlerts){
            const row=document.createElement("div");row.className="alert-row";
            const label=document.createElement("span");
            label.textContent=`${alert.symbol} · ${alert.exchange} · ${alert.direction==="above"?"≥":"≤"} ${formatCurrency(alert.target,alert.currency)}${alert.triggered?" · SEUIL ATTEINT":""}`;
            const del=document.createElement("button");del.type="button";del.className="button";del.textContent="Supprimer";
            del.addEventListener("click",()=>{localAlerts=localAlerts.filter(a=>a.id!==alert.id);storeLocalAlerts();showLocalAlerts();});
            row.append(label,del);container.append(row);
        }
    };
    function checkLocalAlerts(){
        if(!latestMarket || !latestFx?.rates?.USD)return;
        let changed=false;
        for(const alert of localAlerts){
            const tick=latestMarket.symbols?.[alert.symbol]?.[alert.exchange];
            if(tick?.status!=="live" || !Number.isFinite(tick.price))continue;
            const value=convertQuote(tick.price,alert.currency);
            if(!Number.isFinite(value))continue;
            const hit=alert.direction==="above"?value>=alert.target:value<=alert.target;
            if(hit && !alert.triggered){
                alert.triggered=true;changed=true;
                $("alert-notice").textContent=`Seuil local atteint : ${alert.symbol} ${formatCurrency(value,alert.currency)}.`;
            }
            if(!hit && alert.triggered){alert.triggered=false;changed=true;}
        }
        if(changed){storeLocalAlerts();showLocalAlerts();}
    }
    showLocalAlerts();
    $("alert-notice").textContent="Alertes locales actives tant que Pravda-Bot est ouvert, sans ordre de trading ni notification distante.";
    $("price-alert-form").addEventListener("submit", event => {
        event.preventDefault();
        if(localAlerts.length>=30){$("alert-notice").textContent="Limite de 30 alertes locales.";return;}
        const target=Number($("alert-target").value);
        if(!Number.isFinite(target)||target<=0){$("alert-notice").textContent="Seuil positif requis.";return;}
        const alert={
            id:String(Date.now())+"-"+String(Math.random()),
            symbol:$("alert-symbol").value,exchange:$("alert-exchange").value,
            direction:$("alert-direction").value,currency:$("alert-currency").value,
            target,triggered:false
        };
        localAlerts.push(alert);storeLocalAlerts();showLocalAlerts();
        $("alert-notice").textContent="Alerte locale enregistrée. Évaluation uniquement sur des cours récents pendant l'ouverture de l'application.";
    });
    $("history-export").addEventListener("click", async event => {
        event.preventDefault();
        try {
            const query = {
                symbol: $("chart-symbol").value,
                exchange: $("chart-exchange").value,
                period: $("chart-period").value
            };
            const csv = await window.pravdaInvoke("ipc_history_export_csv", { query });
            const blobUrl = URL.createObjectURL(new Blob([csv], { type: "text/csv;charset=utf-8" }));
            const link = document.createElement("a");
            link.href = blobUrl;
            link.download = "market-history.csv";
            document.body.append(link);
            link.click();
            link.remove();
            setTimeout(() => URL.revokeObjectURL(blobUrl), 60000);
        } catch {
            $("history-status").textContent = t("unavailable");
        }
    });
    for (const id of ["daily-report", "ais-geojson"]) {
        const link = $(id);
        link.removeAttribute("href");
        link.setAttribute("aria-disabled", "true");
        link.addEventListener("click", event => event.preventDefault());
    }
    window.addEventListener("resize", drawPriceChart);
    applyLanguage();
    refreshAll();
    function startPolling() {
        setInterval(updateClock, 1000);
        setInterval(refreshMarket, 1000);
        setInterval(refreshHistory, 30000);
        setInterval(refreshFx, 21600000);
        setInterval(refreshFlows, 300000);
        setInterval(refreshNews, 900000);
        setInterval(refreshCrypto, 5000);
        setInterval(refreshSources, 60000);
    }
    startPolling();
    refreshSystem();
    setInterval(refreshSystem, 10000);
    
    // Tabs filtering
    const tabButtons = document.querySelectorAll('.tab-btn');
    const allCards = () => Array.from(document.querySelectorAll('.card[data-category]'));
    function applyTab(tab){
        // Let CSS widen the chosen module instead of keeping empty sidebars.
        document.querySelector(".shell").dataset.activeTab = tab;
        tabButtons.forEach(btn=>btn.classList.toggle('active', btn.dataset.tab===tab));
        const cards = allCards();
        if(tab==='all'){
            cards.forEach(c=>c.classList.remove('hidden'));
        } else {
            cards.forEach(c=>{
                const cat = c.dataset.category;
                if(cat===tab) c.classList.remove('hidden');
                else c.classList.add('hidden');
            });
        }
        // Keep general cards visible
        document.querySelectorAll('.card.secondary-card:not([data-category])').forEach(c=>c.classList.remove('hidden'));
        if (tab === "markets" || tab === "all") {
            requestAnimationFrame(drawPriceChart);
        }
        if (tab === "security") {
            refreshSecurity(true);
            refreshSecurityEvents();
            refreshPaper(true);
        }
    }
    const knownTabs = new Set(Array.from(tabButtons,btn=>btn.dataset.tab));
    const tabFromHash=()=>{const route=window.location.hash.slice(2);return knownTabs.has(route)?route:"all";};
    tabButtons.forEach(btn=>btn.addEventListener("click",()=>{
        const route=btn.dataset.tab;
        window.location.hash="/"+route;
        applyTab(route);
        document.querySelector(".tabs")?.scrollIntoView({block:"nearest",behavior:"smooth"});
        if(route==="revolut")refreshRevolut();
    }));
    window.addEventListener("hashchange",()=>applyTab(tabFromHash()));
    applyTab(tabFromHash());

    // The existing one-second IPC snapshot poll replaces the WebSocket transport.
    // A script load event alone does not prove the event handlers initialized.
    window.pravdaDashboardReady = true;

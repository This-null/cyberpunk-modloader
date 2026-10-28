# CP2077 Mod Launcher

[English](README.md) | [Türkçe](README.tr.md) | [Русский](README.ru.md) | [Español](README.es.md) | [Português (BR)](README.pt-BR.md) | [Français](README.fr.md) | [Polski](README.pl.md) | [Deutsch](README.de.md)

Nieoficjalny menedżer modów społeczności do Cyberpunk 2077 na Windows. Wersja 0.0.4 beta.

Nie jest tworzony przez CD PROJEKT RED, Nexus Mods ani zespół NightCoop i nie jest z nimi powiązany.

## Co robi

- Instaluje mody z plików .zip, .rar i .7z. Upuść plik na okno albo kliknij Zainstaluj mod.
- Włącza mod, kopiując jego pliki do folderu gry, a wyłącza, usuwając dokładnie te pliki. Nadpisane pliki są zapisywane i przywracane.
- Ostrzega, zanim mod nadpisze plik innego moda.
- Znajduje mody zainstalowane ręcznie i pozwala je zaimportować.
- Zbiera kilka wersji jednego moda w jednej karcie, więc widać, która jest aktualna.
- Sprawdza i instaluje wersje NightCoop z oficjalnej strony oraz weryfikuje każde pobranie sumą SHA-256.
- Przegląda i przeszukuje Nexus Mods z filtrami, tagami i nieskończonym przewijaniem. Darmowe konta mogą instalować przez linki nxm albo z folderu pobranych plików.
- Pokazuje, czego wymaga każdy mod z Nexusa, i udostępnia listę modów jako profil (kod lub plik), żeby inni mogli doinstalować brakujące.
- Pokazuje logi gry, modów i launchera oraz czyści je na życzenie.
- Mówi po angielsku, turecku, rosyjsku, hiszpańsku, brazylijskim portugalsku, francusku, polsku i niemiecku.
- Ma motywy: systemowy, ciemny i jasny.
- Instaluje mody FOMOD z oknem opcji, sprawdza CET, RED4ext, ArchiveXL i inne frameworki oraz aktualizuje mody z Premium. Pytania lub pomysły? Wpadnij na nasz [Discord](https://discord.gg/EaNcFyCEqP).

## Instalacja

1. Pobierz plik zip ze strony [Releases](https://github.com/This-null/cyberpunk-modloader/releases).
2. Rozpakuj go i uruchom instalator.
3. Windows może pokazać ostrzeżenie SmartScreen, bo instalator nie jest podpisany. Wybierz Więcej informacji, a potem Uruchom mimo to.

## Prywatność

Nic nie jest wysyłane automatycznie. Raporty błędów zapisują się tylko na Twoim komputerze. Launcher łączy się ze stroną NightCoop i Nexus Mods, żeby wyświetlać i pobierać mody. Odczytuje też z GitHuba numer najnowszej wersji, żeby informować o aktualizacjach. Możesz to wyłączyć w ustawieniach. Notatki do wydania i opisy modów trafiają do Tłumacza Google tylko wtedy, gdy na to pozwolisz w ustawieniach. Udostępnienie profilu online wysyła na nasz serwer tylko listę Twoich modów (źródła i wersje, bez plików) i losowy identyfikator utworzony na Twoim komputerze, tylko po naciśnięciu przycisku, a kod wygasa po 12 godzinach. Twój klucz API Nexusa zostaje w Menedżerze poświadczeń Windows na Twoim komputerze.

## Budowanie ze źródeł

Potrzebujesz Node 20 lub nowszego, pnpm i stabilnego Rusta.

```
pnpm install
pnpm tauri dev
pnpm tauri build
```

## Licencja

MIT, zobacz [LICENSE](LICENSE). Pliki RAR są rozpakowywane kodem UnRAR przez pakiet unrar, który ma własną licencję.

Używaj modów na własne ryzyko i najpierw zrób kopię zapasową zapisów gry.

# CP2077 Mod Launcher

[English](README.md) | [Türkçe](README.tr.md) | [Русский](README.ru.md) | [Español](README.es.md) | [Português (BR)](README.pt-BR.md) | [Français](README.fr.md) | [Polski](README.pl.md) | [Deutsch](README.de.md)

Windows için Cyberpunk 2077'ye yönelik resmi olmayan bir topluluk mod yöneticisi. Sürüm 0.0.5 beta.

CD PROJEKT RED, Nexus Mods veya NightCoop ekibi tarafından yapılmadı ve onlarla bağlantılı değildir.

## Ne yapar

- .zip, .rar ve .7z dosyalarından mod kurar. Dosyayı pencereye bırak ya da Mod kur'a bas.
- Modu açarken dosyalarını oyun klasörüne kopyalar, kapatırken tam olarak o dosyaları siler. Üstüne yazılan dosyalar yedeklenir ve geri konur.
- Bir mod başka bir modun dosyasının üstüne yazacaksa önce uyarır.
- Elle kurduğun modları bulur ve içe aktarmana izin verir.
- Bir modun birden fazla sürümünü tek kartta toplar, hangisinin güncel olduğu görünür.
- NightCoop sürümlerini resmi siteden kontrol edip kurar, her indirmeyi SHA-256 ile doğrular.
- Nexus Mods'ta filtre, etiket ve sonsuz kaydırmayla gezer ve arar. Ücretsiz hesaplar nxm bağlantısı ya da indirilenler klasörü üzerinden kurabilir.
- Her Nexus modunun neye ihtiyaç duyduğunu gösterir ve mod listeni profil olarak (kod ya da dosya) paylaşır, böylece başkaları eksiklerini kurabilir.
- Oyun, mod ve launcher loglarını gösterir, istediğinde temizler.
- İngilizce, Türkçe, Rusça, İspanyolca, Brezilya Portekizcesi, Fransızca, Lehçe ve Almanca konuşur.
- Sistem, koyu ve açık tema vardır.
- FOMOD modlarını seçenek penceresiyle kurar, CET, RED4ext, ArchiveXL gibi çatı modları kontrol eder, Premium hesapta modları günceller. Soru ya da fikrin mi var? [Discord](https://discord.gg/EaNcFyCEqP) sunucumuza gel.

## Kurulum

1. [Releases](https://github.com/This-null/cyberpunk-modloader/releases) sayfasından zip dosyasını indir.
2. Zip'i aç ve kurulum dosyasını çalıştır.
3. Kurulum dosyası kod imzalı olmadığı için Windows SmartScreen uyarısı gösterebilir. Ek bilgi'yi, sonra Yine de çalıştır'ı seç.

## Gizlilik

Hiçbir şey kendiliğinden bir yere gönderilmez. Hata raporları sadece bilgisayarına kaydedilir. Launcher, modları listelemek ve indirmek için NightCoop sitesi ve Nexus Mods ile konuşur. Güncellemeleri haber vermek için GitHub'dan son sürüm numarasını da okur. Bunu Ayarlar'dan kapatabilirsin. Sürüm notları ve mod açıklamaları Google Translate'e sadece Ayarlar'dan izin verirsen gider. Profili çevrimiçi paylaşmak sunucumuza sadece mod listeni (kaynaklar ve sürümler, dosya yok) ve bilgisayarında üretilen rastgele bir kimliği gönderir, sadece düğmeye bastığında, ve kod 12 saat sonra geçersiz olur. Nexus API anahtarın bilgisayarındaki Windows Kimlik Bilgisi Yöneticisi'nde durur.

## Kaynaktan derleme

Node 20 veya üstü, pnpm ve kararlı Rust gerekir.

```
pnpm install
pnpm tauri dev
pnpm tauri build
```

## Lisans

MIT, bkz. [LICENSE](LICENSE). RAR dosyaları unrar paketi üzerinden UnRAR kaynağıyla açılır, onun kendi lisansı vardır.

Modları kendi sorumluluğunda kullan ve önce kayıtlarını yedekle.

# mcloudfast: devir notu

Son güncelleme: 1 Kasım 2026

## Proje ne

mcloudfast, [Spotifast](https://github.com/crmne/spotifast) (Rust + egui ile
yazılmış native bir Spotify istemcisi) üzerine kurulu bir fork. Amacı
[mcloud](https://github.com/bulutakbulutt/mcloud) Spicetify temasını (Apple
Music görünümü) Spotifast'e native bir "arayüz stili" olarak taşımak. Hedef,
Spotify + Spicetify + mcloud ile sayfa sayfa birebir aynı görünüm.

Açmak için: **Ayarlar → Appearance → Interface style → mcloud**.

## Repo

- `origin`: github.com/f0rknturkoglu/mcloudfast (çalışma branch'i `main`)
- `upstream`: crmne/spotifast. Geçmiş tek bir başlangıç commit'ine
  sıkıştırıldı ("Start mcloudfast from Spotifast 0.15.0", upstream
  `796393c`). Bu yüzden upstream güncellemeleri `git merge` ile değil, iki
  upstream sürümü arasındaki fark yama olarak uygulanarak alınır.
- Yerelde `backup/full-history` ve `backup/upstream-main` yedek branch'leri var.
- Uzaktaki `mcloud-ui` branch'i `main` ile aynı ve silinebilir (silme
  komutu izin sisteminde engellendi).

## Kod nerede

mcloud'a ait her şey `src/ui/mcloud/` altında. Spotifast'in kendi
dosyalarına sadece küçük dallanmalar eklendi, böylece upstream çakışmaları az kalıyor.

| Dosya | Ne yapıyor |
|---|---|
| `mod.rs` | `active(ctx)` bayrağı (ortak widget'lar mcloud'u buradan anlar), `sentence_case` |
| `glass.rs` | Liquid glass: GPU paint callback (egui_glow). Tarif mcloud'un `theme.js` → `toolbarLens` (state `player`) kodundan alındı |
| `player.rs` | Yüzen player kapsülü (mcloud `#am-player` ölçüleri) |
| `sidebar.rs` | Apple Music sidebar'ı |
| `topbar.rs` | Üst çubuk (geri/ileri, ev, arama) |
| `pages.rs` | Albüm ve playlist başlığı, Shuffle/Play butonları |
| `artist.rs` | Sanatçı sayfası (başlık görseli, Top Songs ızgarası, raflar) |

Spotifast'te kancaların olduğu yerler:

- **`ui/mod.rs`:** player ve sidebar seçimi, sanatçı rengi, header wash.
- **`collection.rs`:** `hero` ve `actions_row` fonksiyonları.
- **`artist.rs`:** sanatçı sayfası.
- **`topbar.rs`:** `search_box` ortak fonksiyonu.
- **`widgets.rs`:** kartlar, tablo başlığı ve satır çizgileri.
- **`home.rs`:** selamlama başlığı.
- **`theme.rs`:** `Palette::mcloud()` ve `Mcloud*` ikonları.
- **`settings.rs`:** `interface` ve `mcloud_glass` ayarları.
- **`session_reads.rs`:** `PageExtras.header_image`, sanatçı başlık görseli.

## Kurallar

- Ölçüleri tahminle değil, mcloud'un `user.css` ve `theme.js` dosyalarından al.
  mcloud reposu her seferinde yeniden klonlanabilir.
- Yeni her kullanıcı metni **16 dilin hepsine** çevrilmeli
  (`tests/localization.rs` eksik çeviriye izin vermiyor).
  - Önce `src/ui/mcloud/*` dosyasını `assets/i18n/POTFILES` listesine ekle.
  - Sonra `.github/scripts/update-translations.sh` çalıştır ve boş kalan
    `msgstr` alanlarını doldur.
  - `--check` ile doğrula. `cargo fmt` sonrasında satır referansları kaydığı
    için script'i bir kez daha çalıştırmak gerekiyor.
- egui context kilitleri reentrant değil: `ctx.data_mut(|d| …)` içinde başka
  bir `ctx.` çağrısı yapma; deadlock'a sokar. Değeri önce dışarıda oku.
- Kontroller:
  - `cargo fmt --all --check`
  - `cargo clippy --locked --all-targets --all-features -- -D warnings`
  - `cargo test --locked --all-targets`
  - çeviri script'i `--check` ile

## Karşılaştırma nasıl yapılıyor

- **Spotify tarafı:** `open -g spotify:<tür>:<id>` sayfayı arka planda açar.
  Pencere `screencapture -l <pencere-id>` ile yakalanır; pencere kimliği
  CoreGraphics'ten okunur (küçük bir Swift yardımcısıyla).
- **Spotifast tarafı:** `./target/debug/spotifast spotify:<tür>:<id>`,
  çalışan uygulamada aynı sayfayı açar.
- **Örnek veriyle:** `--demo --demo-show mcloud[,glass-full|glass-eco]
  --demo-page <sayfa> --demo-shot out.png` Spotify'a bağlanmadan görüntü alır.
- **Gerçek sayfa kimlikleri:** `~/Library/Caches/me.paolino.spotifast/`
  içindeki JSON dosyalarında.

## Bitenler

Palet ve SF Pro fontu, sidebar, üst çubuk, yüzen player, liquid glass (kullanıcı
memnun), albüm ve playlist başlığı ile butonlar, tablo başlıkları ve satır
çizgileri, sanatçı sayfası, ana sayfa selamlamasının kaldırılması, albüm
sayfasının altı (tam tarih, süre/parça özeti, telif ve "More by [Sanatçı]" rafı),
Liked Songs sayfası (tür çipleri filtreleme ve sağdaki "Recently added ☰" sıralama açılır menüsü),
Ana sayfa (All/Music/Podcasts kategori filtre çipleri, filtrelere göre raf ve kısayol gösterimi,
Spotify'ın 24pt bölüm başlıkları ile raf boşlukları),
Recently Added ve All Playlists kütüphane görünümleri (`Page::RecentlyAdded`, `Page::AllPlaylists`),
Sidebar'a "Son Eklenen" ve "Tüm Listeler" bağlantılarının eklenmesi,
Topbar liquid glass kontrolleri (geri/ileri kapsülü, Home daire butonu ve arama kutusu),
Sidebar glow (mcloud `user.css:206` `#am-sidebar-glow` ile sayfa kapağının/renginin kenardan sızması),
Browse sayfası (`Page::Browse`) ve sidebar "Göz At" bağlantısı.

## Kalanlar (sırayla önerilen)

Sayfaların hepsi güncel mcloud reposuna (bulutakbulutt/mcloud) göre eşitlendi:
- Sanatçı başlığı: isim 40pt; ⓘ 48 / Play 72 / takip 48. Sağ üstte paylaş + "…".
- ⓘ Hakkında kartı (`mcloud/artist.rs`). Veri, Pathfinder artist overview'dan
  `ArtistAbout` olarak geliyor.
- Sanatçı rengi: mcloud'un `artworkColor` + `setTint` algoritması
  (`images::artwork_colours`).
- Kütüphane ızgarası (`mcloud/library.rs`): Recently Added, Artists, Albums,
  All Playlists.
- Arama: Spotify'ın yeni düzeni (`mcloud/search.rs`). Browse: "Browse all" düzeni.
- Sayfa boşluğu `--am-pad`; kenar çubuğu `--am-side`
  (`settings.mcloud_sidebar_width`, yalnızca sürükleyince kaydediliyor).

Web API izin vermediği için yapılamayanlar:
- Browse kategori görselleri.
- Aramadaki "Featuring X" rafı (Spotify'ın editoryal listeleri).
- Liked Songs tür çipleri: sanatçı türleri geliştirici uygulamasına gelmiyor;
  tür gelirse çipler kendiliğinden görünür.

Not: Kullanıcının Spotify'ındaki mcloud eski bir sürüm (ⓘ yok). Karşılaştırmada
fark görülürse önce güncel repoya bak.

## Bilinen durumlar

- Kullanıcının Spotify geliştirici uygulamasının kotası doldu ("Development
  Mode quota is exhausted"). Bu yüzden diskografi gelmiyor; sanatçı
  sayfasında Latest Release ve albüm rafları boş görünüyor. Kod tarafında
  sorun yok.
- Liquid glass referansı: github.com/emribilemir/liquid-glass-research
- Uygulama, `cargo test` ile aynı anda açılırsa macOS anahtar zinciri geç
  yanıt verebilir ve giriş ekranı çıkar ("system credential store did not
  respond"). Testler bitince yeniden açmak yetiyor; oturum silinmiyor.
- Ekran yakalamada Spotify ve Spotifast pencere genişlikleri eşit olmalı:
  mcloud'un boşlukları ve kenar çubuğu pencere genişliğine göre değişiyor.

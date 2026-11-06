# mcloudfast: devir notu

Son güncelleme: 2 Kasım 2026

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
| `lyrics.rs` | Kelime kelime doldurma, Spicy Lyrics glow'u, satır ölçeği, ara noktaları — hem tam ekran hem sağ panel ortak |

Lyrics'e ait diğer dosyalar:

| Dosya | Ne yapıyor |
|---|---|
| `src/lyrics.rs` | Kaynak ayrıştırma (Spotify `color-lyrics`, LRC/enhanced LRC, LRCLIB), `pieces()` zamanları, yazarlar (`writers_from_credits`) |
| `src/ui/lyrics.rs` | Panel ve tam ekran çizimi, `painted_words()`, `writers_line()`, `repaint_for_lyrics()` |
| `src/player.rs` | `lyrics_json()` ve `track_credits_json()` (spclient) |
| `src/backend.rs` | Lirik + yazar akışı; önbellek `spotify-<id>.json` |

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

## Lyrics (2 Kasım 2026)

Sağ panel ve tam ekran aynı yolu kullanıyor (`ui/lyrics.rs` → `mcloud::lyrics`).

**Bitenler.**
- Kelime kelime dolum: kaynak kelime zamanı veriyorsa o (Spotify
  `SYLLABLE_SYNCED`, enhanced LRC `<mm:ss.xx>`), yoksa tahmin.
- Spicy Lyrics'in görselleri (kaynaktan alınan değerlerle):
  - glow: kelime eğrisi 0→0.15→0.6→1 (0→1→1→0), satır eğrisi 0→0.5→1;
    yarıçap satır 0.06–0.32 em, kelime 0.05–0.20 em; 4 halka × 12 kopya,
    kopya alpha 0.06; söylenmiş kelimede 0.28 iz; satırın son kelimesi ×1.5.
    Bloom sadece ışığın ulaştığı kelimelerle sınırlı.
  - pasif satırlar 0.95 ölçekte, söylenen satır yerine büyüyor
    (`lit_line_scale`); sarma genişliği ölçekle bölünüyor, satır yeniden
    sarılmıyor (test: `mcloud_lines_grow_into_place_without_rewrapping`).
  - tek şeritte süpürme: `EDGE = 0.55 em`, 16 şerit; ışık kelime
    boşluklarını kesintisiz geçiyor.
  - ara noktaları: `dot_state()` — 0.75→1.05→1 şişme, −0.12 em yükselme,
    dolumla birlikte 3 halkalı glow.
  - yazarlar: son satırın altında `Written by {names}`, 16 dilde.
- Tahmin modeli: kelimeler `karakter × 85 ms` tempoda dolar; satırın artan
  süresi **son kelimede tutulur** (nota tutma). Dar satırlarda kelimeler eşit
  sıkışır. Uzatılmış ünlü (`loooove`) daha ağır (`drawn_out()`).
- Düzeltilen hata: bir kelime tamamen söylendiğinde f32 glyph genişlik
  toplamı bir milyonluk kısa düşüp ışığı o kelimenin sonunda **kalıcı olarak**
  durduruyordu (`SLACK`), satırın kalanı hiç yanmıyordu.
- `Lyrics.writers` alanı Spotify `songWriters`'tan veya
  spclient `track-credits-view/v0/experimental/{id}/credits` yanıtından
  (`roleCredits`, önce `roleTitle`, yoksa ikinci rol) geliyor; lirik
  önbelleğiyle saklanıyor.
- Sağ panel oynarken kare istemiyordu (takılma); `repaint_for_lyrics()`
  artık kelime doldururken her kareyi, satır yanarken her kareyi, yoksa
  sıradaki satırı istiyor.

**Neden bazı parçalarda tutma kayıyor.** Kaynakta kelime zamanı yok:
önbellekteki 952 satırın tamamı `LINE_SYNCED`. Spotify kelime zamanı
verdiğinde (log: `spotify lyrics for <id> are SYLLABLE_SYNCED`) tutma doğru.
Aynı işi yapan eklentilerin kaynakları:
- Spicy Lyrics: `api.spicylyrics.org` (TTML, kendi sunucusu).
- Liquid Lyrics: Spicy API + `spclient…/color-lyrics/v2/track/`.
- Beautiful Lyrics: `beautiful-lyrics.socalifornian.live/lyrics/{id}`
  (kullanıcının Spotify token'ı ile kendi backend'i).
- Lyrics Plus: Musixmatch RichSync (`apic-appmobile.musixmatch.com`,
  `richsync_compact_type=words`, app token'ı).
Hiçbiri metinden/sesten çıkarmıyor; hepsi gerçek kelime zamanı verisi kullanıyor.
mcloudfast bu üçüncü taraf/hosted kaynaklara bağlanmıyor (repo kuralları:
hosted servis yok, bakım riski).

## Kalanlar (sırayla önerilen)

**Temel problemlerden sonra:** mcloud'un `STATUS.md`'sindeki Spotify ile
Apple Music karşılaştırmaları görev listesine çevrildi:
[MCLOUD-GOREVLER.md](MCLOUD-GOREVLER.md). Temel problemler çözülünce
oradan başla.

Sayfaların hepsi güncel mcloud reposuna (bulutakbulutt/mcloud) göre eşitlendi:
- Sanatçı başlığı: isim 40pt; ⓘ 48 / Play 72 / takip 48. Sağ üstte paylaş + "…".
- ⓘ Hakkında kartı (`mcloud/artist.rs`). Veri, Pathfinder artist overview'dan
  `ArtistAbout` olarak geliyor.
- Sanatçı rengi: mcloud'un `artworkColor` + `setTint` algoritması
  (`images::artwork_colours`).
- Kütüphane ızgarası (`mcloud/library.rs`): Recently Added, Artists, Albums,
  All Playlists.
- Arama: Apple bölümleri (`mcloud/search.rs`): Top Results plakaları, tür rafları, başlıktan açılan "see all" sayfaları (`Page::SearchKind`). Web API Spotify'ın alaka sırasını vermediği için Top Results sanatçı + sırayla türlerden kuruluyor. Browse: "Browse all" düzeni.
- Sayfa boşluğu `--am-pad`; kenar çubuğu `--am-side`
  (`settings.mcloud_sidebar_width`, yalnızca sürükleyince kaydediliyor).

Şimdi Çalan paneli (`mcloud/now_playing.rs`): cam kart ya da kenara yapışık;
kapak, başlık, yıldız, "…" ve altında sözler. Ayarlar: stil, arka plan, sözler.

Lyrics'te kalanlar (Spicy'den alınabilecekler):
- **Pasif satırlarda uzaklık bulanıklığı** (`applyBlur`: aktif satırdan
  uzaklık × 1.25 px, ~6.8 px tavan). egui'de metin blur'u yok; N adet soluk
  kopya pahalı. Şimdilik dikey fade ile idare ediliyor.
- **Harf dalgası** (`RefreshedEmphasis`): harf başına scale +0.10, merkezden
  0.03 em açılma, 0.025 em yukarı, kelime boyunca gecikmeli, min 1 s, son
  kelimede ×1.6. Tam hali harf harf galley ister; ucuz hali söylenen kelimeyi
  büyük puntoyla yeniden dizip üstüne çizmek (pompalama).
- **Duet / arka vokal hizalaması**: `OppositeAligned` + background satırlar.
  Spotify `color-lyrics` bu veriyi vermiyor.
- **Romanizasyon**: Japonca/Korece dönüşüm (Kuromoji/opencc). Büyük iş,
  CJK şarkılarda gerçek kazanç.
- **TTML yan dosyası**: şarkının yanına `.ttml`/`.xml` konursa kelime
  zamanları (ve yazarlar) oradan gelir. `local_tags::lyrics` şu an sadece
  `.lrc` yan dosyasını okuyor; enhanced LRC'nin kelime damgaları zaten
  ayrıştırılıyor.
- **Hece ağırlığı**: denendi, naif ünlü-grubu sayımı sessiz `e` (İngilizce
  "one", "love") ve Türkçe aksanlarda pacing'i bozduğu için geri alındı.
  Bir sözlük/CMUdict olmadan önerilmiyor.
- **Üçüncü taraf provider**: yukarıdaki hosted kaynaklardan biri (opsiyonel
  ayar) — ToS/kırılganlık nedeniyle varsayılan yapılmadı.

Web API izin vermediği için yapılamayanlar:
- Browse kategori görselleri.
- Aramadaki "Featuring X" rafı (Spotify'ın editoryal listeleri).
- Liked Songs tür çipleri: sanatçı türleri geliştirici uygulamasına gelmiyor;
  tür gelirse çipler kendiliğinden görünür.

Not: Kullanıcının Spotify'ındaki mcloud eski bir sürüm (ⓘ yok). Karşılaştırmada
fark görülürse önce güncel repoya bak.

## Bilinen durumlar

- `cargo clean` çalıştırıldı (2 Kasım 2026): `target/` boş, ilk build soğuk
  olacak. Lirik testleri `--features demo` ister
  (`cargo test --features demo --lib lyrics::tests`).
- Lyrics yazarları resmî olmayan `track-credits-view` uç noktasından geliyor;
  ayrıştırma rol başlığına, o yoksa ikinci role bakıyor. Yanıt değişirse
  log'da `spotify credits unavailable` görünür.
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

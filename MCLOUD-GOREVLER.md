# mcloud'dan mcloudfast'a görevler

Son güncelleme: 6 Kasım 2026. Kaynak: mcloud `main` (`c520d2b`), Spotifast tabanı `v0.17.1`.

Bu dosya her mcloud özelliğinin mcloudfast'teki gerçek durumunu tutar. Eski
sürümde 42 maddenin hepsi "Tamamlandı" yazıyordu, oysa bazıları eksikti. O
yüzden artık dört durum var, ve bir maddenin durumu kanıta göre yazılır.

**Durumlar:**
- **Tamam:** Kod var, testi var ya da demo görüntüsünde görüldü.
- **Doğrula:** Kod var, ama gerçek veriyle ya da elle denenmedi.
- **Kısmi:** Bir kısmı var, eksiği yazılı.
- **Yapılamaz:** Sebebi yazılı. Kullanılan kütüphane ya da API izin vermiyor.

**Çalışma şekli**
- Ölçüleri tahminle değil, mcloud'un `user.css` ve `theme.js` dosyalarından al.
- Karar vermeden önce ölç. Hız sorunlarında sebebi tahmin etme.
- Ekran görüntüleri koyu modda alınır, konu light mode değilse.
- Kontroller tıklandığı anda durum değiştirir. İstek sonra gelir.
- "Tamam" yazmadan önce görüntüyle ya da testle doğrula.

## Hemen yapılacak

Yeni işlerin hiçbiri henüz gerçek hesapla denenmedi. Bu tur önce gelir:

- [ ] Gerçek hesapla gezinti: ana sayfa, arama, see-all, Daily Mix kapakları,
      What's New, klasörler, Now Playing, sanatçı sayfası.
- [ ] Web API kotası: dolu ise sayfalama durur, sanatçı mix'leri gradient'e düşer.
- [ ] Windows ve Linux'ta cam ve light mode.
- [ ] `EnableTransparency` okuması Windows'ta çalışıyor mu (kod derlenmedi).

## A. Hız ve akıcılık

| # | Madde | Durum |
|---|---|---|
| 1 | Sanatçı önbelleği ve hover ile ön yükleme | Doğrula |
| 2 | Başlık görselinin en küçük uygun kaynağı | Tamam |
| 3 | Geri/ileri geçişte titreme yok | Doğrula |
| 4 | Gereksiz yeniden çizim yok | Doğrula: raf snap'i düzeltildi, boşta CPU ölçülmedi |
| 5 | Lyrics'te Balanced cam maliyeti | Doğrula: GPU ölçülmedi |

## B. Sanatçı sayfası

| # | Madde | Durum |
|---|---|---|
| 6 | Genişliğe göre düzen | Tamam |
| 7 | Başlık görselinde siyah bant kırpma | Tamam |
| 8 | Başlıkta blur ve renge geçiş | Tamam |
| 9 | Sanatçı rengi, sidebar karışımı | Tamam |
| 10 | Hakkında kartında kaydırma | Doğrula |
| 11 | Takip yıldızı ve toast | Doğrula |
| 43 | Compilations rafı | Tamam |
| 44 | Artist Playlists rafı | Doğrula: gerçek yanıtla karşılaştırılmadı |
| 45 | Upcoming Concerts bölümü ve başlık etiketi | Doğrula: alanlar mcloud kodundan |
| 46 | Sayfa sonunda About bölümü | Doğrula |

## C. Albüm ve playlist sayfaları

| # | Madde | Durum |
|---|---|---|
| 12 | Başlık ölçüleri | Tamam |
| 13 | Parça satırları | Tamam |
| 14 | Yıldız butonu | Tamam |
| 15 | Numara, renk ve başlık ayarları | Tamam |
| 16 | Çalan parça göstergesi | Tamam |
| 17 | Dar alanda aksiyon satırı | Doğrula |
| 18 | Kaydırınca çıkan başlık çubuğu | Doğrula |
| 19 | Yazı görünümü | Doğrula: Music ile yan yana bakılmadı |
| 47 | Kaydırınca yapışan sütun başlığı | Doğrula |
| 48 | Tek ℗ satırı | Tamam |
| 49 | Hide için Apple'ın eksi ikonu | Tamam |
| 50 | "More releases" butonu (#121) | Yapılamaz: Spotify'ın kendi öğesi, bizde "More by" rafı var |

## D. Fotoğraflı başlıklar ve mix kapakları

| # | Madde | Durum |
|---|---|---|
| 20 | Fotoğraf rengi | Tamam |
| 39 | Şerit, istasyon, gradient kapaklar | Doğrula: gerçek Spotify adresleriyle denenmedi |
| 51 | Fotoğraflı playlist kapakla açılır | Tamam |
| 52 | `photoHeaders` kapalıyken üstte fotoğraf | Doğrula: sadece mix'lerde, editoryal listelerde foto verisi yok |
| 53 | "About recommendations" bağlantısı | Yapılamaz: Spotify'ın kendi öğesi |

## E. Sidebar

| # | Madde | Durum |
|---|---|---|
| 21 | Show All / Show Less, kaydırma, satır hareketi | Doğrula |
| 22 | Son çalınan listenin güncellenmesi | Doğrula |
| 23 | "+" ile yerinde playlist | Tamam |
| 23b | Yeni Klasör | Yapılamaz: librespot klasörleri sadece okur |
| 24 | Ölçüler ve yapı, SF Regular satırlar | Tamam |
| 25 | Hesap satırı | Tamam |
| 54 | Klasörlerin gösterimi | Doğrula: klasör sayfası yok, yerinde açılır |
| 55 | Pin ikonu | Kısmi: pinler yerel, Spotify ile paylaşılamaz |
| 56 | Kenara çift tıkla genişliği sıfırla | Doğrula |
| 57 | Son liste sayısı, gösterilen listeler | Tamam |

## F. Arama

| # | Madde | Durum |
|---|---|---|
| 26 | Öneri penceresi | Doğrula |
| 27 | Arama alanı | Tamam |
| 28 | Kütüphane filtresi ve sıralama | Tamam |
| 58 | Apple bölümleri ve see-all (#126) | Doğrula: sayfalama gerçek API'yi görmedi |
| 59 | Top Results sırası | Kısmi: Web API alaka sırası vermiyor, sanatçı ve türler sırayla |
| 60 | Eski sonuçların ekranda kalması | Tamam |
| 61 | Audiobooks | Doğrula: her ülkede yok, gelmezse bölüm çıkmaz |
| 62 | Profiles ve Genres | Yapılamaz: Web API arama türü olarak sunmuyor |

## G. Menüler, paneller, bildirimler

| # | Madde | Durum |
|---|---|---|
| 29 | Menülere cam | Doğrula |
| 30 | Sağ tık menüsü | Doğrula |
| 31 | Player menüleri, ses kapsülü | Doğrula |
| 32 | Sağ panel camı | Doğrula |
| 33 | Toast | Doğrula |
| 41 | Now Playing: kart, panel, yalnızca panel, arka plan | Doğrula: "yalnızca panel" sadece görüntüde bakıldı |
| 63 | Hareketli kapak (`npArt`) | Yapılamaz: egui'de video çözücü yok, Apple seçeneği üçüncü taraf dizine bağlanıyor |
| 64 | Spicy Lyrics kaynağı (`npLyricsSource`) | Yapılamaz: Spicetify eklentisi |
| 65 | Player'dan şarkı ve albüm sürükleme, kütüphane kartı | Doğrula: elle denenmedi |
| 66 | Player'dan sanatçı sürükleme | Yapılamaz: bizde sanatçı bırakılacak hedef yok |
| 67 | Dönen rakamlar | Doğrula |
| 68 | Şarkı önizlemeleri (`previewMaterial`) | Yapılamaz: Spotify'ın watch feed'i, video gerekir |

## H. Cam

| # | Madde | Durum |
|---|---|---|
| 34 | Kenar örnekleme | Doğrula |
| 35 | Toolbar hover, yay, parıltı | Doğrula |
| 36 | Saydamlığı azalt | Kısmi: macOS tamam, Windows kodu derlenmedi |
| 37 | Dithering | Tamam |

## I. Ana sayfa ve diğerleri

| # | Madde | Durum |
|---|---|---|
| 38 | Raf hizası, kırpma, snap, oklar | Doğrula: oka tıklama denenmedi |
| 38b | Rafların sidebar altından kayması | Yapılamaz: mcloud'da da yok |
| 40 | Light mode | Doğrula: demo sayfalarında bakıldı |
| 42 | Playlist filtreleri, menü veya buton | Tamam |
| 69 | Ana sayfa kısayolları ve kartlar (#125) | Tamam |
| 70 | Browse ölçüleri | Tamam |
| 71 | What's New kartları (#128) | Doğrula |
| 72 | What's New Music/Podcast seçimi | Kısmi: istek yalnız albüm alıyor, podcast değeri denenmeden eklenmedi |
| 73 | Kategori görselleri, Liked tür çipleri | Yapılamaz: Web API vermiyor |

## J. Sözler

| # | Madde | Durum |
|---|---|---|
| 74 | Uzaklığa göre soluklaşma | Kısmi: gerçek blur değil, opaklık |
| 75 | TTML yan dosyası | Tamam: parser testli, gerçek dosyayla denenmedi |
| 76 | Harf dalgası | Yapılmadı: harf harf galley ister, karar bekliyor |
| 77 | Duet hizalaması | Yapılamaz: Spotify verisi taşımıyor |
| 78 | Romanizasyon | Yapılmadı: Japonca sözlük gerektirir |

## Gelen kutusu

mcloud'da yeni çıkanlar. `.github/workflows/mcloud-sync.yml` her gün bakar ve
yeni kayıtları buraya ekleyip `main`'e commit eder. Sadece bu dosyaya, sadece
aşağıdaki iki işaretin arasına yazar; işaretleri silme.
Eklenenler: mcloud `main`'e merge edilen PR'lar, tamamlanıp kapanan issue'lar
ve PR'sız `STATUS.md` commit'leri.

Her kaydı yukarıdaki tablolara işle, sonra buradan sil. "tema dosyası yok"
yazan kayıtlar (CI, kurulum, Pages) çoğunlukla mcloudfast'ı ilgilendirmez.
Spicetify'a özgü olanlar (`hashed class`, `Spicetify bindings`) da öyle.

<!-- mcloud-sync:start since=2026-11-06T12:01:58Z -->
<!-- mcloud-sync:end -->

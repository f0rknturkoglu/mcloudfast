# mcloud'dan mcloudfast'a görevler

Son güncelleme: 4 Kasım 2026. Kaynak: mcloud reposunun `STATUS.md` dosyası
(`main`, `34c241d`).

mcloud'da Spotify'ı Apple Music'e benzetirken her sayfayı Apple Music ile
karşılaştırdık; bulunan farklar, ölçüler ve kararlar `STATUS.md`'de. Bu
dosya o sorunları mcloudfast görevlerine çeviriyor.

**Ne zaman:** mcloudfast'ın temel problemleri çözüldükten sonra. Önce onlar.

**Nasıl okunur:**
- Her madde: mcloud'da ne öğrendik → mcloudfast'ta ne yapılacak.
- Sayılar mcloud'un ölçüleri. Tahmin değil, başlangıç noktası.
- **Yok:** mcloudfast kodunda bulunamadı.
- **Kontrol:** bir kısmı var. mcloud ölçüsüyle yan yana karşılaştır, eksikse tamamla.
- Sırası öneri. Önce hız, sonra en çok görülen sayfalar.

**Alınmayanlar:** yalnızca Spotify'ın HTML'ine ya da Spicetify'a ait konular.
jsDelivr, GitHub Pages, Marketplace, React düğümleri, CSS `:has()`
performansı, `Spicetify.GraphQL` gecikmesi, Spotify'ın zoom'u ve Windows'un
60px sürükleme bölgesi bunlara dahil.

## Çalışma şekli (mcloud'da oturan kurallar)

- Karar vermeden önce ölç. Hız sorunlarında sebebi tahmin etme, uygulamada ölç.
- Apple'ın sayıları sadece başlangıç. 2–3 varyantı gerçek pencerede yakala,
  kullanıcı gözüyle seçsin.
- Apple referansı: macOS 27, cam ayarı en net, Music penceresi odakta.
  Gri trafik ışıkları varsa referans geçersiz.
- Ekran görüntüleri koyu modda alınır. Light mode ancak konu light mode ise.
- Kontroller tıklandığı anda durum değiştirir. İstek sonra gelir, hata olursa geri alınır.

## A. Hız ve akıcılık

1. **Sanatçı sayfası önbelleği ve ön yükleme.** *Tamamlandı.*
   mcloud: sayfa verisi oturum boyunca önbellekte tutuluyor. 2 dakika taze
   sayılıyor. 30 dakikaya kadar eski veri gösterilip arkada yenileniyor.
   En fazla 60 kayıt tutuluyor. Bağlantının üstünde 80 ms durunca veri ve
   başlık görseli önceden yükleniyor. Sıcak açılış 63–170 ms, Spotify'ın
   kendi sayfası 116–165 ms.
   Yapılacak: aynı önbellek ve hover ile ön yükleme. Soğuk ve sıcak açılış
   süresini ölç.
2. **Başlık görselinin boyutu.** *Tamamlandı.* Her zaman 2660px'lik görsel
   gelmesin. Başlığı cihazın piksel oranında kaplayan en küçük 7:3 kaynak
   seçilsin.
3. **Geri/ileri geçişinde titreme olmasın.** *Tamamlandı.*
   mcloud'da her geçişte tek bir kare değişiyor (10 sayfa çifti ölçüldü).
   - Görünümler oturum boyunca önbellekte. Arkada yenileniyor, sadece sıra
     değiştiyse yeni hali bir sonraki ziyarette geliyor.
   - "Yükleniyor…" ekranı yanıp sönmüyor.
   - Daha önce yüklenen kapaklar hemen çiziliyor. Son 200 doku bellekte
     tutuluyor.
4. **Gereksiz yeniden çizim olmasın.** *Tamamlandı.* Pencere gizliyken iş
   yapılmasın. Marquee ve scrubber cihaz pikseli adımlarıyla ilerlesin.
   Sadece değer değişince yeniden çizilsin. mcloud'da müzik çalarken boşta
   CPU %27'den %1–5'e indi.
5. **Lyrics sayfasında cam maliyeti.** *Tamamlandı.* Arka plan her kare
   değişince camın altındaki blur da her kare yeniden hesaplanıyor. mcloud'un
   Balanced modu bu sayfada hafif cam kullanıyor: tek örneklemeli bükme, ton
   ve vurgu var; halo ve ince çizgiler yok. GPU kullanımı %98'den %71'e indi.
   mcloudfast'ın `GlassEffect::Balanced` modu lyrics sayfasında ne yapıyor,
   ölç.

## B. Sanatçı sayfası

6. **Düzen pencere genişliğine göre değişsin.** *Yok.* Apple Music'te
   ölçüldü (Nirvana, pencere 980–1386pt):
   - **Dar** (içerik ~600–700pt altı):
     - Top Songs alt alta 5 satır. Başlığın altında "Albüm · Yıl" yazıyor.
     - Öne çıkan kart en üstte, tam genişlikte.
     - Albüm rafında 2 büyük kapak, genişlikle 200'den 275pt'ye büyüyor.
   - **Geniş:**
     - Son çıkan albüm ve Top Songs yan yana.
     - Sütunlar ~220pt. Sağ kenarda bir sonraki sütundan hep ~20pt görünüyor.
     - Fareyle üstüne gelince ">" sayfa oku çıkıyor.
     - Albüm kapakları 135–160pt.
   - Dar pencerede player daha az buton göstersin.
7. **Başlık görselindeki siyah bantları kırp.** *Yok* (`crop()` sadece
   odağa göre kırpıyor). Bazı görsellerde siyah bant var: Doja Cat'te sağda
   ve solda, Sade'de üstte. Görsel 280×120 boyutta örneklenir. Her kanalı
   20'nin altında kalan satır ve sütunlar bant sayılır ve kırpılır. Kırpma
   değeri görsel başına saklanır.
8. **Başlıktaki blur ve geçiş.** *Kontrol.* W, sidebar'ın sağındaki genişlik.
   - Görsel W×W/2 boyutunda.
   - Renge geçiş (α) 0.245W'den 0.646W'ye kadar doğrusal.
   - Blur (σ) 0.38W'ye kadar 0. Sonra 3/6/10/15/20pt
     (0.403/0.432/0.475/0.498/0.52W noktalarında, W=1042 için).
   - Görselin altında bulanık hali devam ediyor (yansıma).
   - Play üçgeninin köşeleri yuvarlak. Üçgen, genişliğinin 1/18'i kadar
     sağda duruyor.
9. **Sanatçı rengi.** *Kontrol.*
   - Gri kapağı olan sanatçının sayfası gri kalsın.
   - Sidebar bu sayfalarda sayfa renginin %34'ünü #161616 ile karıştırır,
     %85 opaklıkla.
   - Kenar parıltısı burada kapalı. Tek renkli sayfayı turuncuya
     çeviriyordu.
10. **ⓘ Hakkında kartında kaydırma.** *Kontrol.*
    - Kaydırınca fotoğraf kaydırma miktarının 0.4 katı kadar kayıyor.
    - 181pt'den (0.257W) başlayarak 200pt boyunca doğrusal olarak bulanıklaşıp
      renge geçiyor. Sonunda σ 36pt.
    - Kart rengi: görselin en sık rengi (24 seviyeli histogramın modu).
      Parlaklığı (HSV V) en fazla 0.42.
    - Etiket, çip ve biyografi metni rengin üstüne ekleniyor (plus-lighter):
      etiket +72, çip +20 dolgu ve +26 kenar, biyografi ilk 150px boyunca
      +187'den +157'ye.
    - Renk sayfa yüklenirken hesaplanmalı. Kart açıldıktan sonra renk
      değişirse titreme olur.
11. **Takip yıldızı.** *Kontrol.* Tıklanınca hemen dolsun, 300 ms küçük bir
    sıçrama yapsın (hareketi azalt ayarı açıksa yapmasın). Toast çıksın:
    "… takip ediliyor" / "… takipten çıkarıldı". İstek başarısız olursa
    yıldız eski haline dönsün.

## C. Albüm ve playlist sayfaları

12. **Başlık ölçüleri.** *Kontrol.* Kaynak: macOS 27 Music.
    - Kapak 150pt ile 270pt arasında, içerik genişliğinin %45'i. Music'te
      hep 270pt; ~1250pt altında başlık kapağın altına iniyor.
    - Kapak köşesi 10px.
    - Başlık 26pt bold. Sanatçı 26pt regular, accent renkte. Satır yüksekliği
      32pt.
    - Yıl satırı 12pt semibold.
    - Playlist açıklaması metin sütununun en altında, 13px, en fazla 3 satır.
13. **Parça satırları.** *Kontrol.*
    - Apple'da satırlar tek satır ve daha ince. Liste metni 13pt.
    - Albüm satırlarında kapak yok. Ayırıcı çizgi 48px'ten başlıyor.
    - Sanatçı satırı olmayan albüm satırında numara dikeyde ortada.
    - Plays sütunu ve video etiketi yok.
    - Sanatçı satırı sadece konuk ya da başka sanatçı olan parçalarda var.
14. **Kaydet butonu yıldız olsun.** *Kontrol.* Music'teki gibi boş ve dolu
    yıldız, accent renkte. Kaydederken Spotify yeşili görünmesin. Shuffle,
    Play ve + da accent renkte.
15. **mcloud'un sayfa ayarları.** *Yok* (ayarlarda sadece arayüz, cam ve
    sidebar genişliği var).
    - Parça numaraları: albüm ve playlist için ayrı ayrı. Gizlenince başlık
      16px sola kayar.
    - Albüm ve playlist rengi: kapalı / hafif / tam. Renk kapaktan gelir,
      light mode'da da çalışır.
    - Parça listesinin başlık satırını gizleme (mcloud B6).
16. **Çalan parçanın göstergesi accent renkte olsun.** *Kontrol.* Spotify'ın
    yeşil ekolayzırı olmasın. Kapağı olan satırda gösterge kapağın üstünde
    durur.
17. **Dar alanda aksiyon satırı.** *Kontrol.* Yer kalmazsa önce sıralama,
    sonra arama gizlensin (mcloud #103).
18. **Kaydırınca çıkan başlık çubuğu.** *Kontrol.* Sayfanın en üstündeyken
    gizli. Play kontrolleri çubuğun altına girince görünür. Cam: #2a2a2a
    %62, blur 30, satürasyon 1.8.
19. **Yazı görünümü.** *Kontrol.* Negatif harf aralığı yok. mcloud bulgusu:
    Chromium'un `antialiased` yumuşatması harfleri Music'ten %25 daha ince
    çiziyordu. egui'nin SF Pro çizimini Music'in ekran görüntüsüyle
    karşılaştır. Referans: "Aynen Öyle", 26pt bold, 2x'te 273×57px.

## D. Fotoğraflı başlıklar (Daily Mix, tür sayfaları)

20. **Başlığın altındaki renk fotoğraftan gelsin.** *Yok* (`header_image`
    sadece sanatçıda var).
    - Renk, fotoğrafın en alttaki onda birinden alınır. Ton korunur,
      satürasyon en fazla %60, açıklık kenarın %60'ı (%10–24 arasında).
    - Fotoğrafın son %38'i bu renge geçer.
    - Renk fotoğraf URL'si başına saklanır (son 100). Yeni bir fotoğrafta
      önceki sayfanın rengi bir an bile görünmez.
    - mcloud'da Spotify'ın kendi rengi kenarla çoğu zaman uyuşmuyordu
      (koyu mor kenarın üstünde parlak yeşil gibi).

## E. Sidebar

21. **Show All / Show Less ve hareketler.** *Kontrol* (başlıkta "Show less"
    var).
    - Liste kapalıyken sadece en altta "Show All" var. Açıkken bu satır
      kalkar. Yerine yapışkan (sticky) başlıkta "Show Less" hapı çıkar. Aynı
      anda iki kontrol olmaz.
    - Açılınca kaydırma konumu değişmez. Odak ilk yeni satıra gider.
    - Listenin derininden kapatınca önce yukarı kayar: en fazla iki sidebar
      yüksekliği, ease-out cubic, mesafeye göre 300–500 ms. Sonra yerinde
      kapanır.
    - Sıra değişince satırlar kayarak yer değiştirir: 300 ms,
      `cubic-bezier(.2,0,0,1)`. Yeni satır 8px düşerek 200 ms'de belirir.
      Hareketi azalt ayarı açıksa hareket yok.
22. **Son çalınanlar ne zaman güncellenir.** *Kontrol.*
    - Çalan liste değişince 0.1 s sonra bakılır. Liste en üste gelene kadar
      0.4 / 1.5 / 5 / 15 s'de tekrar bakılır.
    - Fare ya da klavye odağı listedeyken arka plan yenilemesi bekler.
23. **"+" menüsü ve yerinde adlandırma.** *Kontrol* (Spotifast'te dialog var).
    - "+" menüsünde Yeni Çalma Listesi ve Yeni Klasör var.
    - Ad listenin en üstündeki bir alanda yazılır. Enter ya da alandan
      çıkmak oluşturur, Esc iptal eder.
    - Oluşan liste hemen açılır.
24. **Sidebar ölçüleri ve yapısı.** *Kontrol.*
    - Satır 32px, iç boşluk 16px, ikon 18px, bölüm başlığı 11px.
    - Malzeme: #252525, %76 opak.
    - Parıltının tavanı: 0.4'e kadar olduğu gibi, sonra ~0.66'da yumuşak
      tavan. Büyük renkli alanlar patlamasın.
    - "Mağaza" başlığı yok. "Göz At", Ara ve Ana Sayfa'nın altında, 2x2 ikonla.
25. **Hesap satırı.** *Kontrol.*
    - Playlist'ler altından devam ederken liste son 16px'te soluklaşır ve
      ince bir çizgi görünür. Listenin sonunda ikisi de kalkar.
    - En dar sidebar'da (170px) isim üç noktayla kısalır, butonlar sayfaya
      taşmaz.

## F. Arama

26. **Arama önerileri penceresi.** *Yok.* macOS 27 Music'te ölçüldü.
    - **Pencere:** alanın kenarlarından 1.5pt içeride. Üstü, alanın odak
      halkasının ~1pt altında. Köşeler 13–14pt. Üstte ve altta 5pt boşluk.
      En fazla 523pt yükseklik (3 sorgu + 7 sonuç).
    - **Sorgu satırları:** 24pt. 12.5pt büyüteç kenardan 17.5pt'de. Metin
      38.5pt'den başlar, 13pt. Yazılan kısım %85 beyaz, tamamlanan kısım
      %55.
    - **Sonuç satırları:** 52pt. Kapak 46pt, kenardan 17pt (sanatçıda
      yuvarlak, diğerlerinde ~5pt köşe). Başlık 13pt, alt satır 11pt
      "Şarkı · Sanatçı".
    - **Ayırıcı:** 11pt aralık ve 1pt çizgi (%30 beyaz), 16pt içeride.
    - **Seçim:** accent renkte, kenarlardan 5pt içeride, 6pt köşe, metin
      beyaz. Escape pencereyi kapatır.
    - **Cam:** blur σ ≈ 21pt. Parlaklık 44 + 0.044·L. Renk ×1.14.
    - **Yüklenirken:** aynı camda 3 sorgu ve sonuç iskeleti.
27. **Arama alanı.** *Kontrol.*
    - Büyütecin merceği yuvarlak ucun merkez çizgisinde, 2px sağda.
      Kullanıcı +2px'i +1px'e tercih etti.
    - Metin 36px'ten başlar.
    - Pencere daralınca alan ikona dönüşmez, sadece daralır.
28. **Kütüphane filtresi ve sıralama.** *Kontrol.* Filtre alanı macOS'taki
    gibi yuvarlak ve büyüteçli. Sıralama bir pop-up buton: menü seçili
    öğe butonun üstüne gelecek şekilde açılır.

## G. Menüler, paneller, bildirimler

29. **Menülere cam.** *Yok* (egui menüleri düz).
    - macOS 27 NSMenu'nun en net cam ayarı örnek alınır.
    - Blur 6.8pt.
    - Kenar bükmesi: 3pt'de ~29pt içerideki sayfa görünür. 18pt'den sonra
      bükme yok.
    - Apple'ın tonu: parlak uç sıkışır (beyaz → 146), ton korunur.
    - `glass.rs` buraya genişletilebilir.
    - Satır yüksekliği ve açılış animasyonu kullanıcının onayını bekliyor:
      Apple'da satır 24pt (Spotify'da 40). Apple menüyü animasyonsuz açıyor,
      kapanışta 0.2 s'de soluyor.
30. **Sağ tık menüsü.** *Kontrol.* "Çalma listesine ekle" Spotify'ın kendi
    alt menüsündeki listeleri gösterir: kütüphane sırasında, sadece ekleme
    yapılabilenler. "Sıraya ekle" albüm ve playlist'i parçalarına açarak
    ekler.
31. **Player menüleri ve ses.** *Kontrol.*
    - "Sanatçı — Albüm" satırı "Sanatçıya Git / Albüme Git" menüsünü açar.
      Başlık albüme gider.
    - Ses kapsülü hoparlörün üstünde 0.5 s durunca açılır.
    - Sessize alınca ses seviyesi korunur.
32. **Sağ panel: queue, Friend Activity, cihazlar.** *Yok* (bunlar
    Spotifast'in panelleri).
    - Hepsi tek bir cam yaprakta açılır.
    - Kayarak değil, olduğu yerde belirir. Kayan kenar kesirli piksellerde
      titriyordu.
    - Paneller arasında geçerken Now Playing bir an görünmez.
    - Friends butonu tıklandığı anda basılı görünür.
33. **Toast.** *Tamamlandı.* Player'ın üstünde çıkar, player gibi Liquid Glass.
    Arkasında bir sayfa açık olsa da görünür.

## H. Cam (native'in avantajı olan yerler)

34. **Kenar yapışması ve sol uçtaki pikselleşme.** *Tamamlandı.* Chromium'un
    displacement filtresi örnekleri filtrelemiyor. Bu yüzden kenarın en dış
    ~3 cihaz pikseli yazıların üstünde kırık görünüyordu. Apple filtreliyor.
    mcloud bunu ucuza çözemedi. mcloudfast'ın shader'ında bilinear ya da
    mipmap örnekleme ucuz bir çözüm olabilir. Notlar: liquid-glass-research,
    `research/rim-aa/`.
35. **Toolbar camının davranışı.** *Kontrol.* Her bölümün kendi hover ışığı
    var. Basınca cam yay (spring) hareketiyle büyür. İmlecin etrafında
    parıltı var.
36. **Cam modları ve sistem ayarı.** *Tamamlandı.* Full / Balanced / Eco zaten
    var. macOS ve Windows'un "Saydamlığı azalt" ayarı açıksa Eco'ya geçilsin.
37. **Renk bantlaşması.** *Tamamlandı.* mcloud'da koyu renk geçişlerinde camın
    arkasında 2–4 seviyelik adımlar görünüyordu. Sebep 8 bitlik ara
    sonuçlardı. Shader'da hassasiyeti ve dithering'i kontrol et.

## I. Ana sayfa ve diğerleri

38. **Ana sayfa rafları sidebar'ın altından kaysın.** *Kontrol.* mcloud'da
    yapılamadı, native'de kolay.
    - Shift + tekerlek rafları yatay kaydırır, hafif yapışma (proximity
      snap) ile.
    - Düz dikey tekerleği yataya çevirme. mcloud'da iki kez denendi; sayfa
      ve raflar sürekli birbirinin elinden kaydırmayı alıyordu.
39. **Mix kapakları ve ana sayfa kartları (mcloud #84).** *Kontrol.*
    - Daily Mix ve benzerlerine, görselin kaynak fotoğrafından Apple tarzı
      kapak.
    - Kartlar Apple stilinde. Üstüne gelinen kartta "…" butonu çıkar.
40. **Light mode.** *Kontrol.* `Palette::mcloud()` şu an tek bir palet.
    mcloud'un light mode düzeltmeleri (B2, gölgeler #80) mcloudfast'ta
    karşılığı olmayan bir iş.
41. **Now Playing paneli (mcloud #73).** *Kontrol.* Sağ paneldeki lyrics
    var. Panelin kendisini mcloud ile karşılaştır: kapak, başlık, arka plan
    seçenekleri, kart düzeni.

## Gelen kutusu

mcloud'da yeni çıkanlar. `.github/workflows/mcloud-sync.yml` her gün bakar ve
yeni kayıtları buraya ekleyip `main`'e commit eder. Sadece bu dosyaya, sadece
aşağıdaki iki işaretin arasına yazar; işaretleri silme.
Eklenenler: mcloud `main`'e merge edilen PR'lar, tamamlanıp kapanan issue'lar
ve PR'sız `STATUS.md` commit'leri.

Her kaydı yukarıdaki gruplara görev olarak taşı (Apple Music ölçüleriyle),
sonra buradan sil. "tema dosyası yok" yazan kayıtlar (CI, kurulum, Pages)
çoğunlukla mcloudfast'ı ilgilendirmez.

<!-- mcloud-sync:start since=2026-11-04T19:21:02Z -->
- 2026-11-04 · [PR #112](https://github.com/mcloudHQ/mcloud/pull/112): Playlist filters: By You, By Spotify, Mixed, By Others (#108)
- 2026-11-04 · [Issue #108](https://github.com/mcloudHQ/mcloud/issues/108): \[Feature Request\] Re-implement sorting for playlists
- 2026-11-04 · [PR #113](https://github.com/mcloudHQ/mcloud/pull/113): Spotify's hashed class names, Now Playing slide, library cards (#109, #110, #111) (STATUS.md değişti)
- 2026-11-04 · [Issue #109](https://github.com/mcloudHQ/mcloud/issues/109): Some newly added features are not showing up
<!-- mcloud-sync:end -->

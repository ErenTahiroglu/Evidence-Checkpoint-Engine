# Kurulum ve Geliştirme Ortamı Rehberi (SETUP_TR)

> **Durum:** Bu depo `BOOTSTRAP-00` aşamasında GitHub üzerinde **PUBLIC** olarak ilklendirilmiş, `main` ve `development` dalları oluşturulmuştur.
> Bu rehber, mevcut repository durumunu açıklamakta ve yeni bir geliştirme makinesinde depoyu güvenli şekilde klonlayıp çalışmaya başlama adımlarını sunmaktadır.

## 1. Depo Bilgileri ve Lisans

- **Depo Adı:** `ErenTahiroglu/Evidence-Checkpoint-Engine`
- **Uzak URL:** `https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine.git`
- **Görünürlük:** **PUBLIC** (Açık ve kaynak erişilebilir)
- **Lisans:** **PolyForm Noncommercial License 1.0.0** (Bkz. [LICENSE.md](../LICENSE.md) ve [LICENSING.md](LICENSING.md))
- **Telif Bildirimi:** `Required Notice: Copyright (c) 2026 Eren Tahiroğlu`
- **İki Kalıcı Dal:**
  - `main`: Bağımsız olarak denetlenmiş ve yetkilendirilmiş taban çizgisi (doğrudan commit yasaktır).
  - `development`: Günlük geliştirme, testler, aşama çalışmaları ve dokümantasyon dalı.

---

## 2. Ön Koşullar

- **Git:** Sürüm 2.30 veya üzeri (`git --version`)
- **GitHub CLI (`gh`):** Sürüm 2.0 veya üzeri (`gh --version`)
- **Kimlik Doğrulama:** `gh auth status` komutu geçerli bir oturumu doğrulamalıdır.
- **Git Kullanıcı Bilgisi:** Yerel Git yapılandırmasında `user.name` ve `user.email` tanımlı olmalıdır:
  ```sh
  git config --global user.name "Adınız Soyadınız"
  git config --global user.email "eposta@example.com"
  ```
  *Asla sahte veya başkasına ait kimlik uydurmayınız.*

---

## 3. Yeni Bir Makinede Güvenli Klonlama (Fresh Clone)

Depo bootstrap edildikten sonra yeni bir çalışma ortamında depoyu klonlamak için:

```sh
# 1. Depoyu klonlayın
git clone https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine.git
cd Evidence-Checkpoint-Engine

# 2. development dalına geçin (günlük çalışma dalı)
git checkout development

# 3. Dal ve uzak bağlantı durumunu doğrulayın
git branch -vv
git remote -v
git status
```

---

## 4. Bootstrap Geçmişi ve İlk İskelet (Referans Bilgi)

Bootstrap sırasında gerçekleştirilen yetkili ilklendirme adımları:
1. Dokümantasyon ve lisans iskeleti hazırlandı.
2. `main` dalı üzerinde ilk commit oluşturuldu:
   ```sh
   git commit -m "chore: bootstrap evidence checkpoint engine repository"
   ```
3. `main` dalı uzak depoya gönderildi:
   ```sh
   git push -u origin main
   ```
4. Bu commit üzerinden `development` dalı oluşturuldu ve gönderildi:
   ```sh
   git switch -c development
   git push -u origin development
   ```
5. Her iki uzak dalın aynı SHA değerini işaret ettiği `git ls-remote` ile doğrulandı.

---

## 5. Doğrulama Komutları

Çalışma ortamınızın sağlığını kontrol etmek için şu komutları kullanabilirsiniz:

```sh
# Uzak dalların işaret ettiği SHA değerlerini kontrol edin
git ls-remote --heads origin main development

# Yerel commit SHA değerlerini kontrol edin
git rev-parse main
git rev-parse development

# Çalışma ağacının temiz olduğunu doğrulayın
git status --short
```

---

## 6. Güvenlik ve Katkı Sınırları

- **Asla Hassas Veri Commit Etmeyin:** `.env`, özel anahtarlar (`*.pem`, `*.key`), API token'ları ve kişisel mutlak yollar asla repoya eklenmemelidir. `.gitignore` bu dosyaları hariç tutacak şekilde yapılandırılmıştır.
- **Doğrudan main'e Push Yasaktır:** Tüm geliştirmeler `development` dalında yapılır; `main` dalına terfi yalnızca eksiksiz test kanıtı, bağımsız Red Team incelemesi ve depo sahibinin açık onayı ile gerçekleştirilir.
- **Force Push Yasaktır:** `git push --force` veya geçmişi yeniden yazan operasyonlar kesinlikle yasaktır.
- **Doğrulanmamış Kod Çalıştırmama:** Bootstrap aşamasında çalışan bir doğrulayıcı motor bulunmamaktadır; harici test dosyaları ve arşivler çalıştırılabilir kod değil, veri olarak ele alınmalıdır.

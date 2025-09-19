# MultiversX Wine dApp 🍷

> Aplicație descentralizată pentru industria vinului pe blockchain-ul MultiversX, cu funcționalități complete de registry, marketplace și investment pools.

## 🚀 Caracteristici

### Smart Contracts
- **Wine Registry**: Înregistrarea și certificarea vinurilor ca NFT-uri
- **Wine Marketplace**: Piață descentralizată cu suport pentru licitații
- **Wine Investment Pools**: Pool-uri de investiții colective cu distribuție automată

### Frontend Features
- Interface modernă React cu TypeScript
- Integrare completă cu mx-sdk-dapp v5.0
- Suport pentru Supernova updates (sub-second finality)
- Dashboard interactiv pentru toate funcționalitățile

### Îmbunătățiri Avansate
- 🔐 Sistem de autentificare multi-level
- 📊 Analytics în timp real
- 🌐 Integrare IPFS pentru metadata
- 🎯 Sistem de rating și reviews
- 💰 Yield farming pentru wine tokens
- 📈 Price oracles pentru evaluarea vinurilor
- 🔔 Sistem de notificări push
- 🌍 Suport multi-limbă (RO/EN)

## 📁 Structura Proiectului

```
multiversx-wine-dapp/
├── contracts/              # Smart contracts Rust
│   ├── wine-registry/
│   ├── wine-marketplace/
│   ├── wine-investment/
│   └── wine-governance/    # Nou: Sistem de votare
├── frontend/              # React TypeScript app
│   ├── src/
│   ├── public/
│   └── package.json
├── backend/               # API server Node.js
├── ipfs/                  # IPFS integration
├── oracles/              # Price oracle services
├── deployment/           # Scripts de deployment
├── tests/               # Unit și integration tests
└── docs/               # Documentație completă
```

## 🛠️ Tehnologii Folosite

- **Blockchain**: MultiversX (Elrond) cu Supernova updates
- **Smart Contracts**: Rust + MultiversX SC Framework
- **Frontend**: React 18 + TypeScript + Vite
- **State Management**: Zustand
- **Styling**: Tailwind CSS + Framer Motion
- **Backend**: Node.js + Express + TypeScript
- **Database**: PostgreSQL + Redis
- **Storage**: IPFS pentru metadata și imagini
- **Deployment**: Docker + Kubernetes

## 🚀 Quick Start

1. **Clone repository**:
```bash
git clone https://github.com/Gzeu/multiversx-wine-dapp.git
cd multiversx-wine-dapp
```

2. **Install dependencies**:
```bash
npm run install:all
```

3. **Setup environment**:
```bash
cp .env.example .env
# Configurează variabilele în .env
```

4. **Start development**:
```bash
npm run dev
```

## 📈 Roadmap

- [x] Smart contracts de bază
- [x] Frontend React cu mx-sdk-dapp
- [ ] Sistem de governance și voting
- [ ] Price oracles și yield farming
- [ ] Mobile app React Native
- [ ] Integration cu DeFi protocols
- [ ] Marketplace pentru wine NFTs
- [ ] Analytics dashboard

## 🤝 Contribuții

Contribuțiile sunt binevenite! Te rog să citești [CONTRIBUTING.md](CONTRIBUTING.md) pentru detalii.

## 📄 Licență

Acest proiect este licențiat sub MIT License - vezi [LICENSE](LICENSE) pentru detalii.

## 🔗 Link-uri Utile

- [MultiversX Documentation](https://docs.multiversx.com/)
- [mx-sdk-dapp](https://github.com/multiversx/mx-sdk-dapp)
- [Supernova Updates](https://multiversx.com/releases)

---

**Developed with ❤️ for the wine industry on MultiversX blockchain**
# Migración a JavaScript - Estructura y Especificaciones
// FUENTE: https://deepwiki.com/Loukious/StreamLabsTikTokStreamKeyGenerator
Basado en el código actual del StreamLabs TikTok Stream Key Generator, aquí está la estructura base y especificaciones para migrar a JavaScript:

## Arquitectura Propuesta

### Componentes Principales
- **Frontend**: React/Vue.js para reemplazar PySide6 GUI
- **API Client**: Axios/Fetch para reemplazar requests
- **Authentication**: Manejo OAuth sin Selenium (usar popup/redirect)
- **State Management**: Redux/Context API para reemplazar configuración JSON

## Endpoints de API

Los endpoints críticos a implementar [1](#0-0) :

```javascript
const API_BASE = 'https://streamlabs.com/api/v5/slobs/tiktok';

// Endpoints principales
const ENDPOINTS = {
  INFO: `${API_BASE}/info`,
  SEARCH: `${API_BASE}/info?category={game}`,
  START: `${API_BASE}/stream/start`,
  END: `${API_BASE}/stream/{id}/end`,
  AUTH_DATA: 'https://streamlabs.com/api/v5/slobs/auth/data'
};
```

## Estructura de Clases en JS

### StreamAPI (equivalente a Stream.py)
```javascript
class StreamAPI {
  constructor(token) {
    this.token = token;
    this.client = axios.create({
      baseURL: 'https://streamlabs.com/api/v5/slobs/tiktok',
      headers: {
        'User-Agent': 'Mozilla/5.0...StreamlabsDesktop/1.17.0...',
        'Authorization': `Bearer ${token}`
      }
    });
  }

  async search(game) {
    if (!game) return [];
    const truncatedGame = game.substring(0, 25);
    const response = await this.client.get(`/info?category=${truncatedGame}`);
    return response.data.categories;
  }

  async start(title, category, audienceType = '0') {
    const formData = new FormData();
    formData.append('title', title);
    formData.append('device_platform', 'win32');
    formData.append('category', category);
    formData.append('audience_type', audienceType);
    
    const response = await this.client.post('/stream/start', formData);
    return {
      rtmpUrl: response.data.rtmp,
      streamKey: response.data.key,
      id: response.data.id
    };
  }

  async end(streamId) {
    const response = await this.client.post(`/stream/${streamId}/end`);
    return response.data.success;
  }

  async getInfo() {
    const response = await this.client.get('/info');
    return response.data;
  }
}
```

### AuthManager (equivalente a TokenRetriever.py)
```javascript
class AuthManager {
  constructor() {
    this.CLIENT_KEY = 'awdjaq9ide8ofrtz';
    this.REDIRECT_URI = 'https://streamlabs.com/tiktok/auth';
    this.codeVerifier = this.generateCodeVerifier();
    this.codeChallenge = this.generateCodeChallenge(this.codeVerifier);
  }

  generateCodeVerifier() {
    const array = new Uint8Array(64);
    crypto.getRandomValues(array specifies the OAuth PKCE flow parameters needed for authentication [2](#0-1) );
    return Array.from(array, byte => byte.toString(16).padStart(2, '0')).join('');
  }

  generateCodeChallenge(verifier) {
    const encoder = new TextEncoder();
    const data = encoder.encode(verifier);
    return crypto.subtle.digest('SHA-256', data).then(digest => {
      const decimalArray = Array.from(new Uint8Array(digest));
      const hexadecimalString = decimalArray.map(b => String.fromCharCode(b)).join('');
      const decimalCode = btoa(hexadecimalString);
      return decimalCode.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
    });
  }

  async getAuthUrl() {
    return `https://streamlabs.com/m/login?force_verify=1&external=mobile&skip_splash=1&tiktok&code_challenge=${this.codeChallenge}`;
  }

  async retrieveToken() {
    const params = {
      code_verifier: this.codeVerifier
    };
    const response = await axios.get('https://streamlabs.com/api/v5/slobs/auth/data', { params });
    
    if (response.data.success) {
      return response.data.data.oauth_token;
    }
    throw new Error('Token retrieval failed');
  }
}
```

## Configuración y Estado

### ConfigManager (reemplaza config.json)
```javascript
class ConfigManager {
  constructor() {
    this.config = {
      token: '',
      title: '',
      game: '',
      audienceType: '0',
      suppressDonationReminder: false
    };
  }

  load() {
    const saved = localStorage.getItem('streamlabs-config');
    if (saved) {
      this.config = { ...this.config, ...JSON.parse(saved) };
    }
    return this.config;
  }

  save(config) {
    this.config = { ...this.config, ...config };
    localStorage.setItem('streamlabs-config', JSON.stringify(this.config));
  }
}
```

## Componentes React Sugeridos

### App.js (equivalente a StreamApp)
```javascript
function App() {
  const [config, setConfig] = useState({});
  const [streamAPI, setStreamAPI] = useState(null);
  const [accountInfo, setAccountInfo] = useState({});
  
  useEffect(() => {
    const loadedConfig = configManager.load();
    setConfig(loadedConfig);
    if (loadedConfig.token) {
      const api = new StreamAPI(loadedConfig.token);
      setStreamAPI(api);
      loadAccountInfo(api);
    }
  }, []);

  const loadAccountInfo = async (api) => {
    try {
      const info = await api.getInfo();
      setAccountInfo(info);
    } catch (error) {
      console.error('Failed to load account info:', error);
    }
  };

  return (
    <div className="app">
      <TokenLoader token={config.token} onTokenChange={handleTokenChange} />
      <AccountInfo info={accountInfo} />
      <StreamDetails config={config} onConfigChange={setConfig} />
      <StreamControls streamAPI={streamAPI} config={config} />
    </div>
  );
}
```

## Consideraciones Técnicas

### Manejo de Cookies
- Reemplazar `cookies.json` con `localStorage` o `sessionStorage`
- El manejo de cookies de Selenium se elimina al usar OAuth popup

### Threading
- Reemplazar `threading.Thread()` con `async/await` y Promises
- Las señales de PySide6 se convierten en callbacks/props de React

### Dependencias JS
```json
{
  "dependencies": {
    "axios": "^1.6.0",
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "crypto-js": "^4.0.0"
  }
}
```

## Notas

- La autenticación OAuth en JS será más simple sin Selenium [3](#0-2) 
- Los endpoints de API permanecen iguales, solo cambia el cliente HTTP
- El estado persistente se mueve de archivos JSON a localStorage
- La GUI PySide6 se reemplaza completamente con componentes React/Vue

Wiki pages you might want to explore:
- [Architecture and Components (Loukious/StreamLabsTikTokStreamKeyGenerator)](/wiki/Loukious/StreamLabsTikTokStreamKeyGenerator#3)

### Citations

**File:** Stream.py (L16-44)
```python
        url = f"https://streamlabs.com/api/v5/slobs/tiktok/info?category={game}"
        info = self.s.get(url).json()
        info["categories"].append({"full_name": "Other", "game_mask_id": ""})
        return info["categories"]

    def start(self, title, category, audience_type='0'):
        url = "https://streamlabs.com/api/v5/slobs/tiktok/stream/start"
        files=(
            ('title', (None, title)),
            ('device_platform', (None, 'win32')),
            ('category', (None, category)),
            ('audience_type', (None, audience_type)),
        )
        response = self.s.post(url, files=files).json()
        try:
            self.id = response["id"]
            return response["rtmp"], response["key"]
        except KeyError:
            print("Error: ", response)
            return None, None

    def end(self):
        url = f"https://streamlabs.com/api/v5/slobs/tiktok/stream/{self.id}/end"
        response = self.s.post(url).json()
        return response["success"]
    
    def getInfo(self):
        url = "https://streamlabs.com/api/v5/slobs/tiktok/info"
        response = self.s.get(url).json()
```

**File:** TokenRetriever.py (L11-33)
```python
    CLIENT_KEY = "awdjaq9ide8ofrtz"
    REDIRECT_URI = "https://streamlabs.com/tiktok/auth"
    STREAMLABS_API_URL = "https://streamlabs.com/api/v5/slobs/auth/data"

    def __init__(self, cookies_file='cookies.json'):
        self.code_verifier = self.generate_code_verifier()
        self.code_challenge = self.generate_code_challenge(self.code_verifier)
        self.streamlabs_auth_url = (
            f"https://streamlabs.com/m/login?"
            f"force_verify=1&external=mobile&skip_splash=1&tiktok"
            f"&code_challenge={self.code_challenge}"
        )
        self.cookies_file = cookies_file
        self.auth_code = None

    @staticmethod
    def generate_code_verifier():
        return os.urandom(64).hex()

    @staticmethod
    def generate_code_challenge(code_verifier):
        sha256_hash = hashlib.sha256(code_verifier.encode()).digest()
        return base64.urlsafe_b64encode(sha256_hash).decode("utf-8").rstrip("=")
```

**File:** TokenRetriever.py (L42-89)
```python
    def retrieve_token(self):
        with SB(uc=True, headless=False) as sb:
            sb.open("https://www.tiktok.com/transparency")
            self.load_cookies(sb)

            sb.open(self.streamlabs_auth_url)

            try:
                wait = WebDriverWait(sb, 600)
                wait.until(lambda sb: "success=true" in sb.get_current_url())
            except:
                print("Failed to authorize TikTok.")
                return None
        
        params = {
            'client_key': self.CLIENT_KEY,
            'scope': 'user.info.basic,live.room.tag,live.room.info,live.room.manage,user.info.profile,user.info.stats',
            'aid': '1459',
            'redirect_uri': self.REDIRECT_URI,
            'source': 'web',
            'response_type': 'code'
        }
        with requests.Session() as s:
            try:
                time.sleep(5)
                params= {
                    "code_verifier": self.code_verifier
                }
                response = s.get(self.STREAMLABS_API_URL, params=params)
                if response.status_code != 200:
                    print(f"Bad response: {response.status_code} - {response.text}")
                    return None
                    
                try:
                    resp_json = response.json()
                except json.JSONDecodeError:
                    print("Invalid JSON response. Status code:", response.status_code)
                    return None
                if resp_json.get("success"):
                    token = resp_json["data"].get("oauth_token")
                    print(f"Got Streamlabs OAuth token: {token}")
                    return token
                else:
                    print("Streamlabs token request failed:", resp_json)
                    return None
            except Exception as e:
                print("Error requesting token from Streamlabs:", e)
                return None
```

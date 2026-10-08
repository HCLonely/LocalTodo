import ReactDOM from 'react-dom/client';
import App from './app/App';
import DesktopCard from './app/DesktopCard';
import './styles/theme.css';
import './styles/accessibility.css';
ReactDOM.createRoot(document.getElementById('root')!).render(new URLSearchParams(window.location.search).get('mode')==='card'?<DesktopCard/>:<App/>);

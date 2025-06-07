import { MySorobanProvider } from '@/contexts/sorobanContext';

export default function RootLayout({ children }) {
  return (
    <html lang="en">
      <body>
        <MySorobanProvider>{children}</MySorobanProvider>
      </body>
    </html>
  );
}
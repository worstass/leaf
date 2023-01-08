use winapi::um::winsock2::WSAIoctl;
use winapi::shared::mstcpip::SIO_INDEX_BIND;
use winapi::shared::netioapi::ConvertInterfaceAliasToLuid;

fn a() {


   // WSAIoctl(mSocket, SIO_INDEX_BIND, &anyMulticastInterfaceIndex, sizeof(anyMulticastInterfaceIndex), NULL, 0, &dw, NULL, NULL);

   let a= SIO_INDEX_BIND;
}
import {useEffect,useState} from 'react';
import {apiClient} from '../api/client';
export function useAuth(){
 const [state,setState]=useState(()=>({isAuthenticated:apiClient.isAuthenticated(),isLoading:!apiClient.isInitialized()}));
 useEffect(()=>{const update=()=>setState({isAuthenticated:apiClient.isAuthenticated(),isLoading:!apiClient.isInitialized()});const unsubscribe=apiClient.subscribe(update);void apiClient.restoreSession().then(update);return unsubscribe;},[]);
 const login=async(username:string,password:string,otp='')=>{try{await apiClient.login(username,password,otp);return true;}catch{return false;}};
 return {...state,login,logout:()=>apiClient.logout()};
}

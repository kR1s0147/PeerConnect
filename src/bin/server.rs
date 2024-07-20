use tokio::io::{ReadHalf,WriteHalf};
use tokio::net::{TcpListener,TcpStream};
use tokio::io::{self,AsyncReadExt,AsyncWriteExt};
use core::fmt;
use std::fmt::{format, write};
use std::sync::mpsc::{Receiver, Sender};
use std::{error, string};
use std::{collections::HashMap, io::BufReader, sync::Arc};
use tokio::sync::{Mutex,broadcast};
use std::net::{SocketAddr, ToSocketAddrs};

type db=Arc<Mutex<HashMap<String,String>>>;

#[derive(Debug)]
struct User{
    name:String,
    addr:String,
    // more fields
}
#[derive(Debug)]
enum CustomError{
    UserExists,
    UserIsNotOnline,
    UserDown,

}
impl fmt::Display for CustomError{
    fn fmt(&self, f: & mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CustomError::UserExists => write!(f, "User already exists"),
            CustomError::UserDown=>write!(f, "User is Unreachable"),
            CustomError::UserIsNotOnline => write!(f, "User is not active")
        }
    }
}
impl std::error::Error for CustomError{}


#[tokio::main]
async fn main(){

    let Active_Users:db= Arc::new(Mutex::new(HashMap::new())); 

    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    let (tx,_)=broadcast::channel(20);

    println!("Server listening on port 8080");

    let mut userID=0; 

    while let Ok((stream,addr)) = listener.accept().await {
        println!("Connection accepted! from {:?}",addr);
        let db=Arc::clone(&Active_Users);
        let rx=tx.subscribe();
        let tx1=tx.clone();
        userID+=1;
        let userid=userID;
        tokio::spawn(async   move{
        handle_connection(stream, db,rx,tx1,userid).await;
        });
    }


}

async fn handle_connection( stream:TcpStream,db:db,mut rx:tokio::sync::broadcast::Receiver<String>,tx1:tokio::sync::broadcast::Sender<String> ,userid:i32){
    let mut user:User=User { name:String::new(), addr:String::new() };
    let  (mut rd,mut wrt) = io::split(stream);
    let p=userid.to_string();
    let mut login=false;      
    let mut buf= vec![0;1024];
    let name=p.clone();
    handle_recv(rx,wrt,p.clone()).await;
    loop{ 
       match rd.read(& mut buf).await{
            Ok(n) if n==0 =>{
                println!("Connection closed !");
                match tx1.send("closed".to_string()){
                    Ok(_)=>{}
                    Err(_)=>{
                        println!("connection closed!");
                    }
                }
                removeKey(& mut user, Arc::clone(&db)).await;
                break;
            }
            Ok(n) =>{
                let Message =String::from_utf8_lossy(& buf[..n]).to_string();
                println!("{:?}", Message);
                let parts:Vec<&str>=Message.split(";").collect();
                match parts[0]{
                    "login"=>{
                        if login == true{
                            let s1= format!("{}#Already logged in@",p);
                            tx1.send(s1).unwrap();
                            continue;
                        }
                        let addr=parts[2].to_string();
                        match handle_login(&addr,parts,Arc::clone(&db)).await{
                            Ok(s)=>{
                                if(login==true) {continue;}
                                println!("User logged in :{s}");
                                login=true;
                                user=User{
                                    name: s.clone(),
                                    addr:addr.clone()
                                };
                                let s1=format!("{}#success@",p);
                                tx1.send(s1).unwrap();
                                let name= user.name.clone();
                               let s= format!("{p}#{s}:joined the chat@");
                               tx1.send(s.to_string()).unwrap();
                              
                            }
                            Err(_) =>{
                            }
                        }
                    }
                    "getUser"=>{
                        println!("got get req");
                       match handle_get(& parts,Arc::clone(&db)).await{
                        Ok(s)=>{
                            let s1=format!("{}#{}",p,s);
                           tx1.send(s).unwrap();
                        }
                        Err(_)=>{
                            println!("requested user is not online");   
                            let s= format!("{}#User;Userdown;{}@",p,parts[1]);
                            tx1.send(s).unwrap();
                        }
                       }
                    }
                    _=>{
                        println!("invalid req");
                        let s1= format!("{}# invalid request@",p);
                        tx1.send(s1).unwrap();
                    }
                }
                buf.clear();
                buf.resize(1024, 0);

            }
            Err(e)=>{
                println!("Error occured during the reading the stream {:?}",e);
                tx1.send("closed".to_string()).unwrap();
                removeKey(& mut user, Arc::clone(&db)).await;
                break;
            }

        }
    }
}

async fn handle_recv(mut rx:tokio::sync::broadcast::Receiver<String>,mut wrt:WriteHalf<TcpStream>,name:String){
    tokio::spawn(async move {
        let mut name=name;
        let login=false;
        while let Ok(s)=rx.recv().await{
           if s=="closed"{
            break;
           }
           else{
            if(s.contains("#")){
                println!("{}",s);
                let s1:Vec<& str>=s.split("#").collect();
                if(s1[0]==name){
                    let res=s1[1].to_string();
                    wrt.write_all(res.as_bytes()).await.unwrap();
                }
            }
            else{
            wrt.write_all(s.as_bytes()).await.unwrap();
            }
           }
           
        }
    });
}
 async fn handle_login(addr:&str,parts:Vec<&str>,db:db)-> Result<String,CustomError>{
    let name=parts[1].to_string();
    let mut lock=db.lock().await;
    match lock.get(&name){
            Some(n)=>{
                Err(CustomError::UserExists)
            }
            None =>{
                lock.insert(name.clone(),addr.to_string());
                println!("{} is inserted into db {}",name,addr);
                Ok(name)           
            }
        }
}

async fn removeKey(User : & mut User,db: db){
    let mut  db= db.lock().await;
    let name = & User.name;
    let _=db.remove(name);
}
async fn handle_get(parts:&Vec<&str>,db:db)->Result<String,CustomError>{
   
   let req = parts[1].trim().to_string();
   let mut lock=db.lock().await;
   match lock.get(&req){
    Some(s)=>{
                let res= format!("User;{};{}@",req,s);
                println!("get request is successfull {}",s);
                Ok(res)
            }

    None=>{
        println!("user not found {}",req);
        Err(CustomError::UserIsNotOnline)
    }
    // send user de
   }
}



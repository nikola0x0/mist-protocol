"use client";

import { useState, useEffect, useMemo, useRef } from "react";
import { createPortal } from "react-dom";
import { Typography, Button, Card, useToast } from "@/components/ui";
import { useEnvVars } from "@/hooks";
import { useCurrentAccount, useSignAndExecuteTransaction, useSignPersonalMessage, useSuiClient } from "@mysten/dapp-kit";
import { SuiClient, getFullnodeUrl } from "@mysten/sui/client";
import { Transaction } from "@mysten/sui/transactions";
import { fromHex, toHex } from "@mysten/sui/utils";
import { SealClient, getAllowlistedKeyServers, SessionKey, EncryptedObject } from "@mysten/seal";
import { WalrusClient, TESTNET_WALRUS_PACKAGE_CONFIG } from "@mysten/walrus";
import { batchDecryptQuestions } from "@/lib/question-bank-walrus";
import { uploadQuestionToWalrus } from "@/lib/walrus-upload-with-payment";
import { createQuizContentAPI } from "@/lib/quiz-content-api";
import QuestionForm from "./QuestionForm";
import {
Plus,
Trash2,
Image,
Video,
Music,
Eye,
RefreshCw,
ChevronLeft,
ChevronRight,
Lock,
Unlock,
} from "lucide-react";

interface QuizContent {
id: string;
title: string;
options: string[];
correctAnswer: number;
media?: string;
encryptedBlobId?: string;
sealKeyId?: string;
mediaType: 'video' | 'audio' | 'image';
createdAt: string;
updatedAt: string;
// Runtime fields
isDecrypted?: boolean;
decryptedData?: any;
decryptError?: string;
}

export default function QuestionBankWithSEAL() {
const { envVars } = useEnvVars();
const { showToast } = useToast();
const currentAccount = useCurrentAccount();
const { mutate: signAndExecute } = useSignAndExecuteTransaction();
const { mutate: signPersonalMessage } = useSignPersonalMessage();
const dappKitSuiClient = useSuiClient(); // Use dapp-kit's SuiClient (has CORS configured)

const [questions, setQuestions] = useState<QuizContent[]>([]);
const [loading, setLoading] = useState(true);
const [isFormOpen, setIsFormOpen] = useState(false);
const [previewQuestion, setPreviewQuestion] = useState<QuizContent | null>(null);
const [mounted, setMounted] = useState(false);
const [hasQuestionBank, setHasQuestionBank] = useState(false);
const [questionBankObjectId, setQuestionBankObjectId] = useState<string | null>(null);
const [creatingBank, setCreatingBank] = useState(false);

// Ref to prevent duplicate session key creation (React Strict Mode issue)
const sessionKeyCreationRef = useRef<Promise<void> | null>(null);
const isCreatingSessionKeyRef = useRef<boolean>(false); // Synchronous flag

// SEAL and Walrus clients
const [sealClient, setSealClient] = useState<SealClient | null>(null);
const [walrusClient, setWalrusClient] = useState<any>(null);
const [sessionKey, setSessionKey] = useState<SessionKey | null>(null);
const [suiClient, setSuiClient] = useState<SuiClient | null>(null);

// Pagination
const [currentPage, setCurrentPage] = useState(1);
const questionsPerPage = 5;

const API_URL = (envVars as any).API_URL;
const QUESTION_BANK_PACKAGE_ID = process.env.NEXT_PUBLIC_QUESTION_BANK_PACKAGE_ID || (envVars as any).QUESTION_BANK_PACKAGE_ID;

// Initialize SEAL and Walrus clients
useEffect(() => {
async function initClients() {
if (!currentAccount) return;

      try {
        // Use dapp-kit's SuiClient (has CORS properly configured)
        const suiClient = dappKitSuiClient;
        (suiClient as any).network = 'testnet';

        // Initialize SEAL
        const seal = new SealClient({
          suiClient: suiClient as any,
          serverConfigs: getAllowlistedKeyServers('testnet').map(id => ({
            objectId: id,
            weight: 1,
          })),
          verifyKeyServers: false,
        });

        // Initialize Walrus with upload relay for faster uploads
        const walrus = new WalrusClient({
          suiClient: suiClient as any,
          network: 'testnet',
          uploadRelay: {
            host: 'https://walrus-upload-relay-production.up.railway.app',
            sendTip: null,  // No tip required
          },
        });

        setSealClient(seal);
        setWalrusClient(walrus);
        setSuiClient(suiClient);

      } catch (error) {
        console.error('Failed to initialize Question Bank clients:', error);
      }
    }

    initClients();

}, [currentAccount]);

useEffect(() => {
setMounted(true);
}, []);

// Check Question Bank status
useEffect(() => {
async function checkStatus() {
if (!API_URL || !currentAccount) return;

      try {
        const token = localStorage.getItem('auth_token');
        if (!token) return;

        const response = await fetch(`${API_URL}/api/question-bank/status`, {
          headers: { 'Authorization': `Bearer ${token}` },
        });

        const data = await response.json();
        setHasQuestionBank(data.hasQuestionBank);
        setQuestionBankObjectId(data.questionBankObjectId);

        console.log('Question Bank status:', data);

      } catch (error) {
        console.error('Failed to check Question Bank status:', error);
      }
    }

    checkStatus();

}, [API_URL, currentAccount]);

// Load and auto-decrypt questions
useEffect(() => {
// Prevent duplicate session key creation (React Strict Mode) - synchronous check
if (isCreatingSessionKeyRef.current) {
console.log('⏭️ Session key creation already in progress (sync flag), skipping...');
return;
}

    async function loadAndDecryptQuestions() {
      if (!API_URL || !currentAccount || !sealClient || !walrusClient || !suiClient || !hasQuestionBank) return;

      // Set flag IMMEDIATELY (synchronous)
      isCreatingSessionKeyRef.current = true;

      try {
        setLoading(true);

        const token = localStorage.getItem('auth_token');
        if (!token) {
          showToast({ type: 'error', title: 'Error', message: 'Please login' });
          return;
        }

        // Fetch questions metadata
        const response = await fetch(`${API_URL}/api/question-bank/questions`, {
          headers: { 'Authorization': `Bearer ${token}` },
        });

        if (!response.ok) {
          throw new Error('Failed to fetch questions');
        }

        const questionsList = await response.json();

        if (questionsList.length === 0) {
          setQuestions([]);
          setLoading(false);
          return;
        }

        // Create session key for decryption
        const creationPromise = (async () => {
          const sk = await SessionKey.create({
            address: currentAccount.address,
            packageId: QUESTION_BANK_PACKAGE_ID,
            ttlMin: 10,
            suiClient: suiClient as any,
          });

          const personalMessage = sk.getPersonalMessage();

          await new Promise<void>((resolve, reject) => {
            signPersonalMessage(
              { message: personalMessage },
              {
                onSuccess: async (result: { signature: string }) => {
                  try {
                    await sk.setPersonalMessageSignature(result.signature);
                    setSessionKey(sk);
                    resolve();
                  } catch (error) {
                    reject(error);
                  }
                },
                onError: (error) => {
                  reject(error);
                },
              }
            );
          });

          return sk;
        })();

        sessionKeyCreationRef.current = creationPromise;
        const sk = await creationPromise;
        sessionKeyCreationRef.current = null;
        isCreatingSessionKeyRef.current = false;

        // Auto-decrypt all questions
        const decrypted = await batchDecryptQuestions(
          questionsList,
          {
            questionBankPackageId: QUESTION_BANK_PACKAGE_ID,
            questionBankObjectId: questionBankObjectId!,
            network: 'testnet',
          },
          sealClient,
          walrusClient,
          sk,
          currentAccount.address,  // Pass creator address for transaction sender
          suiClient,  // Pass SuiClient for building transactions
        );

        setQuestions(decrypted);

      } catch (error) {
        console.error('❌ Failed to load questions:', error);
        showToast({
          type: 'error',
          title: 'Error',
          message: 'Failed to load questions',
        });
        isCreatingSessionKeyRef.current = false;  // Clear flag on error
      } finally {
        setLoading(false);
      }
    }

    loadAndDecryptQuestions();

}, [API_URL, currentAccount, sealClient, walrusClient, suiClient, hasQuestionBank, questionBankObjectId]);

// Create Question Bank (one-time)
async function handleCreateQuestionBank() {
if (!currentAccount || !QUESTION_BANK_PACKAGE_ID) {
showToast({ type: 'error', title: 'Error', message: 'Please connect wallet' });
return;
}

    try {
      setCreatingBank(true);

      const tx = new Transaction();
      tx.moveCall({
        target: `${QUESTION_BANK_PACKAGE_ID}::question_bank::create_question_bank_entry`,
        arguments: [
          tx.pure.string(`${currentAccount.address}'s Question Bank`),
        ],
      });

      signAndExecute({
        transaction: tx,
        options: {
          showObjectChanges: true,
          showEffects: true,
        },
      }, {
        onSuccess: async (result: any) => {
          console.log('✅ Question Bank created! Full result:', JSON.stringify(result, null, 2));

          try {
            let bankId: string | undefined;
            let capId: string | undefined;

            // Try multiple extraction methods (dapp-kit format varies)

            // Method 1: objectChanges
            if (result.objectChanges && result.objectChanges.length > 0) {
              console.log('Using objectChanges:', result.objectChanges);
              const bankObj = result.objectChanges.find((obj: any) =>
                obj.type === 'created' && obj.owner === 'Shared'
              );
              const capObj = result.objectChanges.find((obj: any) =>
                obj.type === 'created' && obj.owner?.AddressOwner === currentAccount.address
              );
              bankId = bankObj?.objectId;
              capId = capObj?.objectId;
            }

            // Method 2: effects.created
            if (!bankId && result.effects?.created) {
              console.log('Using effects.created:', result.effects.created);
              for (const obj of result.effects.created) {
                if (obj.owner === 'Shared' || obj.owner?.Shared) {
                  bankId = obj.reference?.objectId || obj.objectId;
                } else if (obj.owner?.AddressOwner === currentAccount.address) {
                  capId = obj.reference?.objectId || obj.objectId;
                }
              }
            }

            // Method 3: Parse from transaction digest (fallback - query blockchain with retry)
            if (!bankId) {
              console.log('Querying blockchain for created objects...');
              const suiClient = new SuiClient({ url: getFullnodeUrl('testnet') });

              // Retry up to 3 times with delay (transaction might not be indexed yet)
              let txResult;
              for (let attempt = 1; attempt <= 3; attempt++) {
                try {
                  if (attempt > 1) {
                    console.log(`Retry ${attempt}/3 after delay...`);
                    await new Promise(resolve => setTimeout(resolve, 2000)); // Wait 2 seconds
                  }

                  txResult = await suiClient.getTransactionBlock({
                    digest: result.digest,
                    options: {
                      showObjectChanges: true,
                      showEffects: true,
                    },
                  });

                  console.log('TX result from blockchain:', txResult);
                  break; // Success!

                } catch (error) {
                  console.warn(`Attempt ${attempt} failed:`, error);
                  if (attempt === 3) throw error; // Give up after 3 tries
                }
              }

              if (txResult) {
                const changes = txResult.objectChanges || [];
                const bankObj = changes.find((obj: any) =>
                  obj.type === 'created' &&
                  (obj.owner === 'Shared' || obj.owner?.Shared) &&
                  obj.objectType?.includes('question_bank::QuestionBank')
                );
                const capObj = changes.find((obj: any) =>
                  obj.type === 'created' &&
                  obj.owner?.AddressOwner === currentAccount.address &&
                  obj.objectType?.includes('question_bank::Cap')
                );

                bankId = bankObj?.objectId;
                capId = capObj?.objectId;
              }
            }

            console.log('Final Bank ID:', bankId);
            console.log('Final Cap ID:', capId);

            if (!bankId || !capId) {
              console.error('Failed to extract object IDs. Full result:', result);

              // Provide manual fallback
              const manualBankId = prompt(
                `Could not auto-extract object IDs. Please check Sui Explorer:\n` +
                `https://testnet.suivision.xyz/txblock/${result.digest}\n\n` +
                `Enter QuestionBank Object ID (Shared object):`
              );

              const manualCapId = prompt('Enter Cap Object ID (owned by you):');

              if (manualBankId && manualCapId) {
                bankId = manualBankId;
                capId = manualCapId;
                console.log('Using manual IDs - Bank:', bankId, 'Cap:', capId);
              } else {
                throw new Error('QuestionBank object IDs required');
              }
            }

            // Save to backend
            const token = localStorage.getItem('auth_token');
            console.log('💾 Saving to backend...', { bankId, capId });

            const saveResponse = await fetch(`${API_URL}/api/question-bank/save-bank-ids`, {
              method: 'POST',
              headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${token}`,
              },
              body: JSON.stringify({
                questionBankObjectId: bankId,
                capObjectId: capId,
              }),
            });

            console.log('Response status:', saveResponse.status);

            if (!saveResponse.ok) {
              const errorText = await saveResponse.text();
              console.error('❌ Failed to save:', errorText);
              throw new Error(`Failed to save: ${saveResponse.status} - ${errorText}`);
            }

            const saveResult = await saveResponse.json();
            console.log('✅ Save result:', saveResult);

            setHasQuestionBank(true);
            setQuestionBankObjectId(bankId);

            showToast({
              type: 'success',
              title: 'Success!',
              message: 'Question Bank created successfully',
            });

            // Force reload to show questions
            window.location.reload();

          } catch (error) {
            console.error('Failed to save bank IDs:', error);
            showToast({
              type: 'error',
              title: 'Error',
              message: 'Failed to save Question Bank IDs',
            });
          }
        },
        onError: (error) => {
          console.error('❌ Failed to create Question Bank:', error);
          showToast({
            type: 'error',
            title: 'Error',
            message: 'Failed to create Question Bank',
          });
        },
      });

    } catch (error) {
      console.error('Error creating Question Bank:', error);
      showToast({ type: 'error', title: 'Error', message: 'Failed to create Question Bank' });
    } finally {
      setCreatingBank(false);
    }

}

// Add question (frontend encrypts + uploads, creator pays WAL!)
async function handleAddQuestion(questionData: any) {
if (!currentAccount || !sealClient || !walrusClient || !questionBankObjectId) {
showToast({ type: 'error', title: 'Error', message: 'Not ready' });
return;
}

    try {

      // Step 1: Prepare question data
      let videoBase64: string | null = null;
      if (questionData.mediaFile) {
        const videoBuffer = await questionData.mediaFile.arrayBuffer();
        videoBase64 = btoa(
          new Uint8Array(videoBuffer).reduce((data, byte) => data + String.fromCharCode(byte), '')
        );
      }

      const fullQuestionData = {
        title: questionData.title,
        options: questionData.options,
        correctAnswer: questionData.correctAnswer,
        video: videoBase64,
        mediaType: questionData.mediaType,
      };

      // Step 2: Generate encryption ID (using SEAL example pattern)
      const nonce = crypto.getRandomValues(new Uint8Array(5));
      const policyObjectBytes = fromHex(questionBankObjectId);  // Use fromHex from Sui utils!
      const encryptionId = toHex(new Uint8Array([...policyObjectBytes, ...nonce]));  // Use toHex!

      // Encrypt with SEAL
      const encryptionSealClient = new SealClient({
        suiClient: suiClient as any,
        serverConfigs: getAllowlistedKeyServers('testnet').map((id) => ({
          objectId: id,
          weight: 1,
        })),
        verifyKeyServers: false,
      });

      const { encryptedObject } = await encryptionSealClient.encrypt({
        threshold: 2,
        packageId: QUESTION_BANK_PACKAGE_ID,
        id: encryptionId,
        data: new TextEncoder().encode(JSON.stringify(fullQuestionData)),
      });

      // Verify encryption has key servers
      const encryptedObjParsed = EncryptedObject.parse(encryptedObject);
      if (encryptedObjParsed.services.length === 0) {
        throw new Error('Encryption failed - no key servers embedded');
      }

      // Upload to Walrus (creator pays WAL)

      const { blobId, blobObjectId } = await uploadQuestionToWalrus(
        fullQuestionData,
        encryptedObject,
        walrusClient,
        currentAccount.address,
        signAndExecute,
      );

      // Save metadata to backend
      const token = localStorage.getItem('auth_token');
      const response = await fetch(`${API_URL}/api/question-bank/add-question`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${token}`,
        },
        body: JSON.stringify({
          title: questionData.title,
          options: questionData.options,
          correctAnswer: questionData.correctAnswer,
          mediaType: questionData.mediaType,
          encryptedBlobId: blobId,
          sealKeyId: encryptionId,
        }),
      });

      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(`Failed to save metadata: ${response.status} - ${errorText}`);
      }

      await response.json();

      showToast({
        type: 'success',
        title: 'Success!',
        message: 'Question added (you paid WAL for storage)',
      });

      // Refresh to show new question
      window.location.reload();

    } catch (error) {
      console.error('❌ Failed to add question:', error);
      showToast({
        type: 'error',
        title: 'Error',
        message: error instanceof Error ? error.message : 'Failed to add question',
      });
    }

}

// Delete question
async function handleDeleteQuestion(questionId: string) {
if (!confirm('Are you sure?')) return;

    try {
      const token = localStorage.getItem('auth_token');
      await fetch(`${API_URL}/api/question-bank/questions/${questionId}`, {
        method: 'DELETE',
        headers: { 'Authorization': `Bearer ${token}` },
      });

      setQuestions(prev => prev.filter(q => q.id !== questionId));

      showToast({
        type: 'success',
        title: 'Deleted',
        message: 'Question deleted successfully',
      });

    } catch (error) {
      console.error('Failed to delete:', error);
      showToast({ type: 'error', title: 'Error', message: 'Failed to delete' });
    }

}

// Pagination
const totalPages = Math.ceil(questions.length / questionsPerPage);
const startIndex = (currentPage - 1) \* questionsPerPage;
const endIndex = startIndex + questionsPerPage;
const currentQuestions = questions.slice(startIndex, endIndex);

// Render
if (!currentAccount) {
return (
<Card className="p-8 text-center">
<Typography variant="h3" weight="bold" className="mb-4">
🔒 Question Bank
</Typography>
<Typography color="secondary">
Please connect your wallet to access the Question Bank
</Typography>
</Card>
);
}

if (!hasQuestionBank) {
return (
<Card className="p-8 text-center">
<Typography variant="h3" weight="bold" className="mb-4">
📚 Question Bank
</Typography>
<Typography color="secondary" className="mb-6">
You don't have a Question Bank yet. Create one to start saving questions!
</Typography>
<Button
          variant="primary"
          size="lg"
          onClick={handleCreateQuestionBank}
          disabled={creatingBank}
        >
{creatingBank ? '⏳ Creating...' : '✨ Create Question Bank'}
</Button>
<Typography variant="small" color="secondary" className="mt-4">
This is a one-time setup. Your questions will be encrypted with SEAL.
</Typography>
</Card>
);
}

if (loading) {
return (
<div className="space-y-6">
<div className="flex justify-between items-center">
<Typography variant="h3" weight="bold">
🔓 Decrypting Questions...
</Typography>
</div>
<Card className="p-8 text-center">
<Typography color="secondary">
Loading and decrypting your questions with SEAL...
</Typography>
</Card>
</div>
);
}

return (
<div className="space-y-6">
<div className="flex justify-between items-center">
<div>
<Typography variant="h3" weight="bold" className="mb-2">
📚 Question Bank
</Typography>
<Typography variant="body" color="secondary">
{questions.length} questions | Encrypted with SEAL
</Typography>
</div>
<div className="flex items-center gap-2">
<Button
variant="outline"
size="lg"
onClick={() => window.location.reload()}
className="flex items-center gap-2" >
<RefreshCw className="w-5 h-5" />
Refresh
</Button>
<Button
variant="primary"
size="lg"
onClick={() => setIsFormOpen(true)}
className="flex items-center gap-2" >
<Plus className="w-5 h-5" />
Add Question
</Button>
</div>
</div>

      {questions.length === 0 ? (
        <Card className="text-center py-12">
          <Typography variant="body" color="secondary" className="mb-4">
            No questions yet. Add your first question!
          </Typography>
        </Card>
      ) : (
        <>
          <div className="space-y-4">
            {currentQuestions.map((question) => (
              <Card key={question.id} className="overflow-hidden">
                <div className="px-8 py-6 bg-gray-50 border-b">
                  <div className="flex items-start justify-between">
                    <div className="flex-1">
                      <div className="flex items-center gap-2 mb-2">
                        {question.isDecrypted ? (
                          <Unlock className="w-5 h-5 text-green-600" />
                        ) : (
                          <Lock className="w-5 h-5 text-red-600" />
                        )}
                        <Typography variant="h4" weight="bold">
                          {question.title}
                        </Typography>
                      </div>
                      {!question.isDecrypted && question.decryptError && (
                        <Typography variant="small" className="text-red-600">
                          ❌ {question.decryptError}
                        </Typography>
                      )}
                    </div>
                    <div className="flex gap-2">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => setPreviewQuestion(question)}
                        disabled={!question.isDecrypted}
                        className="p-2"
                      >
                        <Eye className="w-4 h-4" />
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => handleDeleteQuestion(question.id)}
                        className="p-2 text-red-600"
                      >
                        <Trash2 className="w-4 h-4" />
                      </Button>
                    </div>
                  </div>
                </div>

                {question.isDecrypted && (
                  <div className="p-6">
                    <div className="grid grid-cols-2 gap-2">
                      {question.options.map((option, idx) => {
                        const isCorrect = idx === question.correctAnswer;
                        return (
                          <div
                            key={idx}
                            className={`p-3 border-2 rounded ${
                              isCorrect
                                ? 'bg-green-100 border-green-600 font-bold'
                                : 'bg-gray-100 border-gray-300'
                            }`}
                          >
                            {option}
                          </div>
                        );
                      })}
                    </div>
                  </div>
                )}
              </Card>
            ))}
          </div>

          {/* Pagination */}
          {totalPages > 1 && (
            <Card className="mt-6">
              <div className="p-4 flex items-center justify-between">
                <Typography variant="body" color="secondary">
                  Page {currentPage} of {totalPages}
                </Typography>
                <div className="flex gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setCurrentPage(p => Math.max(1, p - 1))}
                    disabled={currentPage === 1}
                  >
                    <ChevronLeft className="w-4 h-4" />
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setCurrentPage(p => Math.min(totalPages, p + 1))}
                    disabled={currentPage === totalPages}
                  >
                    <ChevronRight className="w-4 h-4" />
                  </Button>
                </div>
              </div>
            </Card>
          )}
        </>
      )}

      {/* Question Form Modal */}
      {isFormOpen && mounted && createPortal(
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4 z-[10000]">
          <Card className="w-full max-w-2xl bg-white">
            <div className="p-6">
              <div className="flex justify-between items-center mb-4">
                <Typography variant="h3" weight="bold">
                  Add Question
                </Typography>
                <Button variant="outline" size="sm" onClick={() => setIsFormOpen(false)}>
                  ✕
                </Button>
              </div>
              <QuestionForm
                onSubmit={handleAddQuestion}
                onCancel={() => setIsFormOpen(false)}
              />
            </div>
          </Card>
        </div>,
        document.body
      )}

      {/* Preview Modal */}
      {previewQuestion && mounted && createPortal(
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4 z-[10000]">
          <Card className="w-full max-w-xl bg-white shadow-2xl">
            <div className="p-6">
              <div className="flex justify-between items-center mb-4">
                <Typography variant="h4" weight="bold">
                  Question Preview
                </Typography>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setPreviewQuestion(null)}
                >
                  ✕
                </Button>
              </div>

              <div className="space-y-4">
                <Typography variant="h4" weight="medium">
                  {previewQuestion.title}
                </Typography>

                {/* Media preview */}
                {previewQuestion.decryptedData?.video && (
                  <div>
                    {previewQuestion.mediaType === 'video' && (
                      <video
                        src={`data:video/mp4;base64,${previewQuestion.decryptedData.video}`}
                        controls
                        className="w-full max-h-64"
                      />
                    )}
                    {previewQuestion.mediaType === 'image' && (
                      <img
                        src={`data:image/jpeg;base64,${previewQuestion.decryptedData.video}`}
                        alt="Question media"
                        className="w-full max-h-64 object-contain"
                      />
                    )}
                    {previewQuestion.mediaType === 'audio' && (
                      <audio
                        src={`data:audio/mp3;base64,${previewQuestion.decryptedData.video}`}
                        controls
                        className="w-full"
                      />
                    )}
                  </div>
                )}

                <div className="grid grid-cols-2 gap-2">
                  {previewQuestion.options.map((option, idx) => {
                    const isCorrect = idx === previewQuestion.correctAnswer;
                    return (
                      <div
                        key={idx}
                        className={`p-3 border-2 rounded ${
                          isCorrect
                            ? 'bg-green-100 border-green-600 font-bold'
                            : 'bg-gray-100 border-gray-300'
                        }`}
                      >
                        {option} {isCorrect && '✓'}
                      </div>
                    );
                  })}
                </div>
              </div>
            </div>
          </Card>
        </div>,
        document.body
      )}
    </div>

);
}
